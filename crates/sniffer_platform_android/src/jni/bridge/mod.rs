pub mod apps;
pub mod icons;
pub mod wallpaper;

pub use apps::{
    get_application_list, init_app_list_cache, open_default_home_picker, request_permissions,
};
pub use icons::{
    get_app_icon_pixels, init_icon_worker_pool, poll_async_app_icon, prefetch_app_icons,
    request_async_app_icon,
};
pub use wallpaper::{
    get_system_wallpaper_pixels, poll_async_wallpaper, request_system_wallpaper_async,
};

use jni::errors::Error as JniError;
use jni::{Env, jni_sig, jni_str};
use jni::{
    JavaVM,
    objects::{JObject, JValue},
};
use std::sync::{Arc, OnceLock};

static JVM: OnceLock<Arc<JavaVM>> = OnceLock::new();

/// Returns the cached global `JavaVM` instance, initialising it on first call.
pub(super) fn vm() -> Arc<JavaVM> {
    if let Some(vm) = JVM.get() {
        return Arc::clone(vm);
    }

    let context = ndk_context::android_context();
    let vm = unsafe { JavaVM::from_raw(context.vm().cast::<jni::sys::JavaVM>()) };

    let arc_vm = Arc::new(vm);
    let stored_vm = JVM.get_or_init(|| arc_vm);

    Arc::clone(stored_vm)
}

/// Returns the Android context as a `JObject` tied to the given `Env` lifetime.
pub(super) fn context<'local>(env: &Env<'local>) -> JObject<'local> {
    let android_context = ndk_context::android_context();
    unsafe { JObject::from_raw(env, android_context.context() as jni::sys::jobject) }
}

/// Adds `FLAG_ACTIVITY_NEW_TASK` to the provided intent.
///
/// # Errors
///
/// Returns an error if the flag cannot be read or applied through JNI.
pub fn add_new_task_flag(env: &mut Env, intent: &JObject<'_>) -> Result<(), String> {
    // FLAG_ACTIVITY_NEW_TASK (0x10000000) | FLAG_ACTIVITY_RESET_TASK_IF_NEEDED (0x00200000)
    let flag = 0x1020_0000i32;

    env.call_method(
        intent,
        jni_str!("addFlags"),
        jni_sig!("(I)Landroid/content/Intent;"),
        &[JValue::Int(flag)],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Starts an Android activity using the provided context and intent.
///
/// # Errors
///
/// Returns an error if `startActivity` fails through JNI.
pub fn start_activity(
    env: &mut Env,
    context: &JObject<'_>,
    intent: &JObject<'_>,
) -> Result<(), String> {
    env.call_method(
        context,
        jni_str!("startActivity"),
        jni_sig!("(Landroid/content/Intent;)V"),
        &[intent.into()],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Returns Android `DisplayMetrics` (density, scaledDensity).
///
/// Returns `(1.0, 1.0)` as a safe fallback if the JNI call fails.
#[must_use]
pub fn get_density() -> (f32, f32) {
    let jvm = vm();

    jvm.attach_current_thread_for_scope::<_, _, JniError>(|env: &mut Env| {
        let ctx = context(env);

        let resources = env
            .call_method(
                &ctx,
                jni_str!("getResources"),
                jni_sig!("()Landroid/content/res/Resources;"),
                &[],
            )?
            .l()?;

        let display_metrics = env
            .call_method(
                &resources,
                jni_str!("getDisplayMetrics"),
                jni_sig!("()Landroid/util/DisplayMetrics;"),
                &[],
            )?
            .l()?;

        let density = env
            .get_field(&display_metrics, jni_str!("density"), jni_sig!("F"))?
            .f()?;

        let scaled_density = env
            .get_field(&display_metrics, jni_str!("scaledDensity"), jni_sig!("F"))?
            .f()?;

        Ok((density, scaled_density))
    })
    .unwrap_or((1.0_f32, 1.0_f32))
}

/// Sets `FLAG_SHOW_WALLPAPER` on the Activity's window so the system wallpaper
/// is composited behind the (transparent-cleared) OpenGL surface by the OS compositor.
/// This is the official Android API for launchers and works on all API levels.
pub fn set_show_wallpaper_flag(app: &android_activity::AndroidApp) {
    let jvm = vm();
    let _ = jvm.attach_current_thread_for_scope::<_, _, JniError>(|env: &mut Env| {
        let activity_ptr = app.activity_as_ptr() as jni::sys::jobject;
        if activity_ptr.is_null() {
            return Err(JniError::NullPtr("activity"));
        }
        let activity = unsafe { JObject::from_raw(env, activity_ptr) };
        if activity.is_null() {
            return Err(JniError::NullPtr("activity jobject"));
        }

        // window.addFlags(WindowManager.LayoutParams.FLAG_SHOW_WALLPAPER = 0x00100000)
        let window = env
            .call_method(
                &activity,
                jni_str!("getWindow"),
                jni_sig!("()Landroid/view/Window;"),
                &[],
            )?
            .l()?;

        let _ = env.call_method(
            &window,
            jni_str!("addFlags"),
            jni_sig!("(I)V"),
            &[JValue::Int(0x0010_0000i32)], // FLAG_SHOW_WALLPAPER
        );
        let _ = env.exception_clear();

        // window.setFormat(PixelFormat.TRANSLUCENT = -3)
        let _ = env.call_method(
            &window,
            jni_str!("setFormat"),
            jni_sig!("(I)V"),
            &[JValue::Int(-3i32)],
        );
        let _ = env.exception_clear();

        // window.setBackgroundDrawable(null)
        let _ = env.call_method(
            &window,
            jni_str!("setBackgroundDrawable"),
            jni_sig!("(Landroid/graphics/drawable/Drawable;)V"),
            &[JValue::Object(&jni::objects::JObject::null())],
        );
        let _ = env.exception_clear();

        Ok(())
    });
}
