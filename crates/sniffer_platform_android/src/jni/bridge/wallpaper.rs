#![allow(dead_code)]

use super::icons::extract_drawable_pixels;
use super::{context, vm};
use jni::errors::Error as JniError;
use jni::{Env, jni_sig, jni_str};
use jni::objects::JValue;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

pub struct WallpaperLoadResult {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

static WALLPAPER_RES_SENDER: OnceLock<crossbeam_channel::Sender<WallpaperLoadResult>> = OnceLock::new();
static WALLPAPER_RES_RECEIVER: OnceLock<crossbeam_channel::Receiver<WallpaperLoadResult>> = OnceLock::new();
static WALLPAPER_FETCHING: AtomicBool = AtomicBool::new(false);

fn init_wallpaper_channel() {
    if WALLPAPER_RES_SENDER.get().is_none() {
        let (tx, rx) = crossbeam_channel::unbounded();
        let _ = WALLPAPER_RES_SENDER.set(tx);
        let _ = WALLPAPER_RES_RECEIVER.set(rx);
    }
}

fn get_wallpaper_cache_dir_path(env: &mut Env) -> std::path::PathBuf {
    let ctx = context(env);
    let path_res: Result<String, JniError> = (|| {
        let files_dir = env.call_method(&ctx, jni_str!("getCacheDir"), jni_sig!("()Ljava/io/File;"), &[])?.l()?;
        let path_obj = env.call_method(&files_dir, jni_str!("getAbsolutePath"), jni_sig!("()Ljava/lang/String;"), &[])?.l()?;
        let path_jstring = env.as_cast::<jni::objects::JString>(&path_obj)?;
        let path_str = path_jstring.try_to_string(env)?;
        Ok(path_str)
    })();

    let base = path_res.unwrap_or_else(|_| "/data/data/com.greenkod.snifferlauncher/cache".to_string());
    let dir = std::path::PathBuf::from(base).join("wallpaper");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn load_wallpaper_from_disk(env: &mut Env) -> Option<(Vec<u8>, u32, u32)> {
    let dir = get_wallpaper_cache_dir_path(env);
    let path = dir.join("wallpaper.raw");
    if let Ok(data) = std::fs::read(&path) {
        if data.len() >= 8 {
            let width = u32::from_le_bytes(data[0..4].try_into().ok()?);
            let height = u32::from_le_bytes(data[4..8].try_into().ok()?);
            let expected = 8 + (width as usize) * (height as usize) * 4;
            if data.len() == expected {
                let pixels = data[8..].to_vec();
                return Some((pixels, width, height));
            }
        }
        let _ = std::fs::remove_file(path);
    }
    None
}

fn save_wallpaper_to_disk(env: &mut Env, pixels: &[u8], width: u32, height: u32) {
    let dir = get_wallpaper_cache_dir_path(env);
    let path = dir.join("wallpaper.raw");
    let tmp_path = dir.join("wallpaper.tmp");
    let mut data = Vec::with_capacity(8 + pixels.len());
    data.extend_from_slice(&width.to_le_bytes());
    data.extend_from_slice(&height.to_le_bytes());
    data.extend_from_slice(pixels);
    if std::fs::write(&tmp_path, data).is_ok() {
        let _ = std::fs::rename(&tmp_path, path);
    }
}

/// Requests the system wallpaper asynchronously without blocking the UI thread.
pub fn request_system_wallpaper_async(target_w: u32, target_h: u32) {
    init_wallpaper_channel();
    if WALLPAPER_FETCHING.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(move || {
        if let Some((pixels, w, h)) = get_system_wallpaper_pixels(target_w, target_h) {
            if let Some(sender) = WALLPAPER_RES_SENDER.get() {
                let _ = sender.send(WallpaperLoadResult { pixels, width: w, height: h });
            }
        }
        WALLPAPER_FETCHING.store(false, Ordering::SeqCst);
    });
}

/// Non-blocking check for async loaded wallpaper result.
#[must_use]
pub fn poll_async_wallpaper() -> Option<WallpaperLoadResult> {
    if let Some(receiver) = WALLPAPER_RES_RECEIVER.get() {
        receiver.try_recv().ok()
    } else {
        None
    }
}

/// Retrieves the system wallpaper from Android WallpaperManager as a raw RGBA pixel buffer.
/// Returns `Option<(pixels, width, height)>`.
#[must_use]
pub fn get_system_wallpaper_pixels(target_w: u32, target_h: u32) -> Option<(Vec<u8>, u32, u32)> {
    let jvm = vm();

    jvm.attach_current_thread_for_scope::<_, _, JniError>(|env: &mut Env| {
        if let Some(cached) = load_wallpaper_from_disk(env) {
            return Ok(cached);
        }

        let ctx = context(env);

        let wp_mgr_cls = match env.find_class(jni_str!("android/app/WallpaperManager")) {
            Ok(cls) => cls,
            Err(e) => {
                let _ = env.exception_clear();
                return Err(e);
            }
        };

        let wp_mgr = match env.call_static_method(
            wp_mgr_cls,
            jni_str!("getInstance"),
            jni_sig!("(Landroid/content/Context;)Landroid/app/WallpaperManager;"),
            &[JValue::Object(&ctx)],
        ) {
            Ok(val) => val.l()?,
            Err(e) => {
                let _ = env.exception_clear();
                return Err(e);
            }
        };

        let drawable = match env.call_method(
            &wp_mgr,
            jni_str!("getDrawable"),
            jni_sig!("()Landroid/graphics/drawable/Drawable;"),
            &[],
        ) {
            Ok(val) => val.l()?,
            Err(_) => {
                let _ = env.exception_clear();
                match env.call_method(
                    &wp_mgr,
                    jni_str!("peekDrawable"),
                    jni_sig!("()Landroid/graphics/drawable/Drawable;"),
                    &[],
                ) {
                    Ok(val) => val.l()?,
                    Err(e2) => {
                        let _ = env.exception_clear();
                        return Err(e2);
                    }
                }
            }
        };

        if drawable.is_null() {
            let _ = env.exception_clear();
            return Err(JniError::JavaException);
        }

        let w = if target_w == 0 { 540 } else { (target_w / 2).clamp(360, 720) };
        let h = if target_h == 0 { 960 } else { (target_h / 2).clamp(640, 1280) };

        let rgba_bytes = match extract_drawable_pixels(env, &drawable, w.cast_signed(), h.cast_signed()) {
            Ok(bytes) => bytes,
            Err(e) => {
                let _ = env.exception_clear();
                return Err(e);
            }
        };

        save_wallpaper_to_disk(env, &rgba_bytes, w, h);

        Ok((rgba_bytes, w, h))
    })
    .ok()
}


