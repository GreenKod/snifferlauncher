use super::icons::extract_drawable_pixels;
use super::{context, vm};
use jni::errors::Error as JniError;
use jni::{Env, jni_sig, jni_str};
use jni::objects::JValue;

/// Retrieves the system wallpaper from Android WallpaperManager as a raw RGBA pixel buffer.
/// Returns `Option<(pixels, width, height)>`.
#[must_use]
pub fn get_system_wallpaper_pixels(target_w: u32, target_h: u32) -> Option<(Vec<u8>, u32, u32)> {
    let jvm = vm();

    jvm.attach_current_thread_for_scope::<_, _, JniError>(|env: &mut Env| {
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

        Ok((rgba_bytes, w, h))
    })
    .ok()
}
