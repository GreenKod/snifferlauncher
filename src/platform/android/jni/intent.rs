use android_activity::AndroidApp;
use jni::Env;

use super::bridge::{add_new_task_flag, context, start_activity, vm};

/// Returns the top and bottom safe-area insets in pixels for the given Android window.
#[must_use]
pub fn get_safe_area(app: &AndroidApp) -> Option<(i32, i32)> {
    use jni::{jni_sig, jni_str};

    let jvm = vm();

    jvm.attach_current_thread_for_scope::<_, _, jni::errors::Error>(|env: &mut Env| {
        let activity_ptr = app.activity_as_ptr() as jni::sys::jobject;
        if activity_ptr.is_null() {
            return Ok(None);
        }

        let activity = unsafe { jni::objects::JObject::from_raw(env, activity_ptr) };
        if activity.is_null() {
            return Ok(None);
        }

        let window = env
            .call_method(
                &activity,
                jni_str!("getWindow"),
                jni_sig!("()Landroid/view/Window;"),
                &[],
            )?
            .l()?;

        let decor_view = env
            .call_method(
                &window,
                jni_str!("getDecorView"),
                jni_sig!("()Landroid/view/View;"),
                &[],
            )?
            .l()?;

        let insets = env
            .call_method(
                &decor_view,
                jni_str!("getRootWindowInsets"),
                jni_sig!("()Landroid/view/WindowInsets;"),
                &[],
            )?
            .l()?;

        if insets.is_null() {
            return Ok(None);
        }

        let top = env
            .call_method(
                &insets,
                jni_str!("getSystemWindowInsetTop"),
                jni_sig!("()I"),
                &[],
            )?
            .i()?;

        let bottom = env
            .call_method(
                &insets,
                jni_str!("getSystemWindowInsetBottom"),
                jni_sig!("()I"),
                &[],
            )?
            .i()?;

        Ok(Some((top, bottom)))
    })
    .ok()
    .flatten()
}

/// Launches a specific Android system or application action.
///
/// # Errors
///
/// Returns an error if the Android JNI calls fail or if the requested action
/// cannot be started by the system.
pub fn launch_action(action: crate::core::types::Action) -> Result<(), String> {
    use crate::core::types::Action;
    match action {
        Action::OpenSettings => start_action("android.settings.SETTINGS"),
        Action::OpenContacts => start_view_uri("content://contacts/people"),
        Action::OpenCamera => start_action("android.media.action.STILL_IMAGE_CAMERA"),
    }
}

fn start_action(action: &str) -> Result<(), String> {
    use jni::{jni_sig, jni_str};

    let jvm = vm();

    jvm.attach_current_thread_for_scope::<_, _, jni::errors::Error>(|env: &mut Env| {
        let ctx = context(env);

        let action_string = env.new_string(action)?;
        let intent = env.new_object(
            jni_str!("android/content/Intent"),
            jni_sig!("(Ljava/lang/String;)V"),
            &[(&action_string).into()],
        )?;

        add_new_task_flag(env, &intent).map_err(|_| jni::errors::Error::JavaException)?;
        start_activity(env, &ctx, &intent).map_err(|_| jni::errors::Error::JavaException)?;

        Ok(())
    })
    .map_err(|e| e.to_string())
}

/// Opens the provided URI with the Android VIEW intent.
///
/// # Errors
///
/// Returns an error if URI parsing, intent construction, or activity launch fails.
pub fn start_view_uri(uri: &str) -> Result<(), String> {
    use jni::{jni_sig, jni_str};
    use jni::objects::JValue;

    let jvm = vm();

    jvm.attach_current_thread_for_scope::<_, _, jni::errors::Error>(|env: &mut Env| {
        let ctx = context(env);

        let action_string = env.new_string("android.intent.action.VIEW")?;
        let uri_string = env.new_string(uri)?;

        let parsed_uri = env
            .call_static_method(
                jni_str!("android/net/Uri"),
                jni_str!("parse"),
                jni_sig!("(Ljava/lang/String;)Landroid/net/Uri;"),
                &[(&uri_string).into()],
            )?
            .l()?;

        let intent = env.new_object(
            jni_str!("android/content/Intent"),
            jni_sig!("(Ljava/lang/String;Landroid/net/Uri;)V"),
            &[(&action_string).into(), (&parsed_uri).into()],
        )?;

        add_new_task_flag(env, &intent).map_err(|_| jni::errors::Error::JavaException)?;
        start_activity(env, &ctx, &intent).map_err(|_| jni::errors::Error::JavaException)?;

        Ok(())
    })
    .map_err(|e| e.to_string())
}
