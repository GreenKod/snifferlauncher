pub mod apps;
pub mod icons;
pub mod wallpaper;

pub use apps::{get_application_list, init_app_list_cache, open_default_home_picker, request_permissions};
pub use icons::{
    get_app_icon_pixels, init_icon_worker_pool, poll_async_app_icon, prefetch_app_icons,
    request_async_app_icon,
};
pub use wallpaper::get_system_wallpaper_pixels;

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
    let flag = env
        .get_static_field(
            jni_str!("android/content/Intent"),
            jni_str!("FLAG_ACTIVITY_NEW_TASK"),
            jni_sig!("I"),
        )
        .and_then(jni::JValueOwned::i)
        .map_err(|e| e.to_string())?;

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
