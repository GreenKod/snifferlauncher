use super::{context, vm};
use crossbeam_channel::{Receiver, Sender, unbounded};
use jni::errors::Error as JniError;
use jni::objects::JValue;
use jni::{Env, jni_sig, jni_str};
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};
use std::thread;

pub struct IconLoadRequest {
    pub package_name: String,
}

pub struct IconLoadResult {
    pub package_name: String,
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

static ICON_REQ_SENDER: OnceLock<Sender<IconLoadRequest>> = OnceLock::new();
static ICON_RES_RECEIVER: OnceLock<Receiver<IconLoadResult>> = OnceLock::new();
static PENDING_REQUESTS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

fn pending_requests() -> &'static Mutex<HashSet<String>> {
    PENDING_REQUESTS.get_or_init(|| Mutex::new(HashSet::new()))
}

/// Initializes the 4-worker JNI async icon decoding channel pool.
pub fn init_icon_worker_pool() {
    if ICON_REQ_SENDER.get().is_some() {
        return;
    }

    let (req_tx, req_rx) = unbounded::<IconLoadRequest>();
    let (res_tx, res_rx) = unbounded::<IconLoadResult>();

    let _ = ICON_REQ_SENDER.set(req_tx);
    let _ = ICON_RES_RECEIVER.set(res_rx);

    for _ in 0..4 {
        let req_rx_clone = req_rx.clone();
        let res_tx_clone = res_tx.clone();

        thread::spawn(move || {
            let jvm = vm();
            let _ = jvm.attach_current_thread::<_, (), JniError>(|env: &mut Env| {
                // Prepare a Looper for this thread because some OEM's AdaptiveIconDrawables
                // require a Looper to run animations or resolve state.
                if let Ok(looper_class) = env.find_class(jni_str!("android/os/Looper")) {
                    let _ = env.call_static_method(
                        looper_class,
                        jni_str!("prepare"),
                        jni_sig!("()V"),
                        &[],
                    );
                }

                while let Ok(req) = req_rx_clone.recv() {
                    let _ = env.with_local_frame(128, |env| {
                        if let Some((pixels, width, height)) =
                            get_app_icon_pixels_inner(env, &req.package_name)
                        {
                            let _ = res_tx_clone.send(IconLoadResult {
                                package_name: req.package_name.clone(),
                                pixels,
                                width,
                                height,
                            });
                        }
                        Ok::<(), JniError>(())
                    });
                    if let Ok(mut pending) = pending_requests().lock() {
                        pending.remove(&req.package_name);
                    }
                }
                Ok(())
            });
        });
    }
}

/// Request asynchronous loading of an application icon.
pub fn request_async_app_icon(package_name: &str) {
    if let Ok(mut pending) = pending_requests().lock() {
        if !pending.insert(package_name.to_string()) {
            return;
        }
    }

    if let Some(sender) = ICON_REQ_SENDER.get() {
        let _ = sender.send(IconLoadRequest {
            package_name: package_name.to_string(),
        });
    }
}

/// Enqueues a list of package names to the async JNI icon worker pool ahead of time.
pub fn prefetch_app_icons(package_names: &[String]) {
    for pkg in package_names {
        request_async_app_icon(pkg);
    }
}

/// Try to receive completed async icon decode results (non-blocking).
pub fn poll_async_app_icon() -> Option<IconLoadResult> {
    if let Some(receiver) = ICON_RES_RECEIVER.get() {
        receiver.try_recv().ok()
    } else {
        None
    }
}

/// Retrieves the application icon for a given package name and returns it as a raw RGBA pixel buffer.
pub(crate) fn extract_drawable_pixels(
    env: &mut Env,
    drawable: &jni::objects::JObject,
    width: i32,
    height: i32,
) -> Result<Vec<u8>, JniError> {
    // Attempt to mutate the drawable so it doesn't share state (fixes some OEM icon issues)
    let _ = env.call_method(
        drawable,
        jni_str!("mutate"),
        jni_sig!("()Landroid/graphics/drawable/Drawable;"),
        &[],
    );
    let _ = env.exception_clear(); // It's fine if mutate fails

    let config_class = env.find_class(jni_str!("android/graphics/Bitmap$Config"))?;
    let argb8888 = env
        .get_static_field(
            config_class,
            jni_str!("ARGB_8888"),
            jni_sig!("Landroid/graphics/Bitmap$Config;"),
        )?
        .l()?;

    // Get DisplayMetrics to ensure AdaptiveIconDrawable scales properly on custom ROMs
    let ctx = super::context(env);
    let display_metrics = env
        .call_method(
            &ctx,
            jni_str!("getResources"),
            jni_sig!("()Landroid/content/res/Resources;"),
            &[],
        )
        .and_then(|res| {
            env.call_method(
                &res.l()?,
                jni_str!("getDisplayMetrics"),
                jni_sig!("()Landroid/util/DisplayMetrics;"),
                &[],
            )
        })
        .and_then(|dm| dm.l());

    let bitmap_class = env.find_class(jni_str!("android/graphics/Bitmap"))?;

    let bitmap = if let Ok(dm) = display_metrics {
        env.call_static_method(
            bitmap_class,
            jni_str!("createBitmap"),
            jni_sig!("(Landroid/util/DisplayMetrics;IILandroid/graphics/Bitmap$Config;)Landroid/graphics/Bitmap;"),
            &[
                JValue::Object(&dm),
                JValue::Int(width),
                JValue::Int(height),
                JValue::Object(&argb8888),
            ],
        )?
        .l()?
    } else {
        let _ = env.exception_clear();
        env.call_static_method(
            bitmap_class,
            jni_str!("createBitmap"),
            jni_sig!("(IILandroid/graphics/Bitmap$Config;)Landroid/graphics/Bitmap;"),
            &[
                JValue::Int(width),
                JValue::Int(height),
                JValue::Object(&argb8888),
            ],
        )?
        .l()?
    };

    let canvas_class = env.find_class(jni_str!("android/graphics/Canvas"))?;
    let canvas = env.new_object(
        canvas_class,
        jni_sig!("(Landroid/graphics/Bitmap;)V"),
        &[JValue::Object(&bitmap)],
    )?;

    // Ensure the canvas is clear
    let _ = env.call_method(
        &canvas,
        jni_str!("drawColor"),
        jni_sig!("(I)V"),
        &[JValue::Int(0)],
    );
    let _ = env.exception_clear();

    // Force the drawable to be visible and fully opaque (fixes Tecno/MIUI silent draw failure)
    let _ = env.call_method(
        drawable,
        jni_str!("setAlpha"),
        jni_sig!("(I)V"),
        &[JValue::Int(255)],
    );
    let _ = env.call_method(
        drawable,
        jni_str!("setVisible"),
        jni_sig!("(ZZ)Z"),
        &[JValue::Bool(true), JValue::Bool(false)],
    );
    let _ = env.exception_clear();

    env.call_method(
        drawable,
        jni_str!("setBounds"),
        jni_sig!("(IIII)V"),
        &[
            JValue::Int(0),
            JValue::Int(0),
            JValue::Int(width),
            JValue::Int(height),
        ],
    )?;

    env.call_method(
        drawable,
        jni_str!("draw"),
        jni_sig!("(Landroid/graphics/Canvas;)V"),
        &[JValue::Object(&canvas)],
    )?;

    let pixel_count = usize::try_from(width).unwrap_or(0) * usize::try_from(height).unwrap_or(0);
    let pixels_array = env.new_int_array(pixel_count)?;

    env.call_method(
        &bitmap,
        jni_str!("getPixels"),
        jni_sig!("([IIIIIII)V"),
        &[
            JValue::Object(&pixels_array),
            JValue::Int(0),
            JValue::Int(width),
            JValue::Int(0),
            JValue::Int(0),
            JValue::Int(width),
            JValue::Int(height),
        ],
    )?;

    let mut buf = vec![0i32; pixel_count];
    pixels_array.get_region(env, 0, &mut buf)?;

    let rgba_bytes: Vec<u8> = buf
        .into_iter()
        .flat_map(|pixel| {
            let [alpha, red, green, blue] = pixel.cast_unsigned().to_be_bytes();
            vec![red, green, blue, alpha]
        })
        .collect();

    Ok(rgba_bytes)
}

fn get_icon_cache_dir_path(env: &mut Env) -> std::path::PathBuf {
    let ctx = context(env);
    let path_res: Result<String, JniError> = (|| {
        let files_dir = env
            .call_method(
                &ctx,
                jni_str!("getCacheDir"),
                jni_sig!("()Ljava/io/File;"),
                &[],
            )?
            .l()?;
        let path_obj = env
            .call_method(
                &files_dir,
                jni_str!("getAbsolutePath"),
                jni_sig!("()Ljava/lang/String;"),
                &[],
            )?
            .l()?;
        let path_jstring = env.as_cast::<jni::objects::JString>(&path_obj)?;
        let path_str = path_jstring.try_to_string(env)?;
        Ok(path_str)
    })();

    let base =
        path_res.unwrap_or_else(|_| "/data/data/com.greenkod.snifferlauncher/cache".to_string());
    let dir = std::path::PathBuf::from(base).join("icons");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn load_icon_from_disk(env: &mut Env, package_name: &str) -> Option<(Vec<u8>, u32, u32)> {
    let dir = get_icon_cache_dir_path(env);
    let path = dir.join(format!("{package_name}.raw"));
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

fn save_icon_to_disk(env: &mut Env, package_name: &str, pixels: &[u8], width: u32, height: u32) {
    let dir = get_icon_cache_dir_path(env);
    let path = dir.join(format!("{package_name}.raw"));
    let tmp_path = dir.join(format!("{package_name}.tmp"));
    let mut data = Vec::with_capacity(8 + pixels.len());
    data.extend_from_slice(&width.to_le_bytes());
    data.extend_from_slice(&height.to_le_bytes());
    data.extend_from_slice(pixels);
    if std::fs::write(&tmp_path, data).is_ok() {
        let _ = std::fs::rename(&tmp_path, path);
    }
}

/// Retrieves the application icon for a given package name and returns it as a raw RGBA pixel buffer.
/// Returns `Option<(pixels, width, height)>`.
#[must_use]
pub fn get_app_icon_pixels(package_name: &str) -> Option<(Vec<u8>, u32, u32)> {
    let jvm = vm();

    jvm.attach_current_thread_for_scope::<_, _, JniError>(|env: &mut Env| {
        match get_app_icon_pixels_inner(env, package_name) {
            Some(res) => Ok(res),
            None => Err(JniError::JavaException),
        }
    })
    .ok()
}

pub(crate) fn get_app_icon_pixels_inner(
    env: &mut Env,
    package_name: &str,
) -> Option<(Vec<u8>, u32, u32)> {
    if let Some(cached) = load_icon_from_disk(env, package_name) {
        return Some(cached);
    }

    let result: Result<(Vec<u8>, u32, u32), JniError> = (|| {
        let ctx = context(env);

        let pm = match env.call_method(
            &ctx,
            jni_str!("getPackageManager"),
            jni_sig!("()Landroid/content/pm/PackageManager;"),
            &[],
        ) {
            Ok(val) => val.l()?,
            Err(e) => {
                let _ = env.exception_clear();
                return Err(e);
            }
        };

        let pkg_str = env.new_string(package_name)?;

        let drawable = match env.call_method(
            &pm,
            jni_str!("getApplicationIcon"),
            jni_sig!("(Ljava/lang/String;)Landroid/graphics/drawable/Drawable;"),
            &[JValue::Object(&pkg_str)],
        ) {
            Ok(val) => val.l()?,
            Err(_) => {
                let _ = env.exception_clear();
                let app_info = match env.call_method(
                    &pm,
                    jni_str!("getApplicationInfo"),
                    jni_sig!("(Ljava/lang/String;I)Landroid/content/pm/ApplicationInfo;"),
                    &[JValue::Object(&pkg_str), JValue::Int(128)], // GET_META_DATA
                ) {
                    Ok(val) => val.l()?,
                    Err(e2) => {
                        let _ = env.exception_clear();
                        return Err(e2);
                    }
                };

                match env.call_method(
                    &pm,
                    jni_str!("getApplicationIcon"),
                    jni_sig!(
                        "(Landroid/content/pm/ApplicationInfo;)Landroid/graphics/drawable/Drawable;"
                    ),
                    &[JValue::Object(&app_info)],
                ) {
                    Ok(val) => val.l()?,
                    Err(e3) => {
                        let _ = env.exception_clear();
                        return Err(e3);
                    }
                }
            }
        };

        if drawable.is_null() {
            let _ = env.exception_clear();
            return Err(JniError::JavaException);
        }

        // Target standard launcher grid resolution (96x96) for optimal VRAM usage and fast JNI extraction.
        let (width, height) = (96, 96);

        let rgba_bytes = match extract_drawable_pixels(env, &drawable, width, height) {
            Ok(bytes) => bytes,
            Err(e) => {
                let _ = env.exception_clear();
                return Err(e);
            }
        };

        let w = width.cast_unsigned();
        let h = height.cast_unsigned();
        save_icon_to_disk(env, package_name, &rgba_bytes, w, h);

        Ok((rgba_bytes, w, h))
    })();

    result.ok()
}
