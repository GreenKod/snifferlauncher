use crate::core::app::Action;
use android_activity::AndroidApp;
use jni::{
    objects::{JObject, JValue},
    AttachGuard, JavaVM,
};
use std::sync::OnceLock;

static JVM: OnceLock<JavaVM> = OnceLock::new();

pub fn launch_action(action: Action) -> Result<(), String> {
    match action {
        Action::OpenSettings => start_action("android.settings.SETTINGS"),
        Action::OpenContacts => start_view_uri("content://contacts/people"),
        Action::OpenCamera => start_action("android.media.action.STILL_IMAGE_CAMERA"),
    }
}

pub fn get_safe_area(app: &AndroidApp) -> Option<(i32, i32)> {
    let mut env = env().ok()?;
    let activity = unsafe { JObject::from_raw(app.activity_as_ptr() as jni::sys::jobject) };

    if activity.is_null() {
        return None;
    }

    let window = env
        .call_method(&activity, "getWindow", "()Landroid/view/Window;", &[])
        .ok()?
        .l()
        .ok()?;
    let decor_view = env
        .call_method(&window, "getDecorView", "()Landroid/view/View;", &[])
        .ok()?
        .l()
        .ok()?;
    let insets = env
        .call_method(
            &decor_view,
            "getRootWindowInsets",
            "()Landroid/view/WindowInsets;",
            &[],
        )
        .ok()?
        .l()
        .ok()?;

    if insets.is_null() {
        return None;
    }

    let top = env
        .call_method(&insets, "getSystemWindowInsetTop", "()I", &[])
        .ok()?
        .i()
        .ok()?;
    let bottom = env
        .call_method(&insets, "getSystemWindowInsetBottom", "()I", &[])
        .ok()?
        .i()
        .ok()?;

    Some((top, bottom))
}

fn start_action(action: &str) -> Result<(), String> {
    let mut env = env()?;
    let context = context();
    let action = env.new_string(action).map_err(to_string)?;

    let intent = env
        .new_object(
            "android/content/Intent",
            "(Ljava/lang/String;)V",
            &[JValue::Object(&action)],
        )
        .map_err(to_string)?;

    add_new_task_flag(&mut env, &intent)?;
    start_activity(&mut env, &context, &intent)
}

fn start_view_uri(uri: &str) -> Result<(), String> {
    let mut env = env()?;
    let context = context();
    let action = env
        .new_string("android.intent.action.VIEW")
        .map_err(to_string)?;
    let uri = env.new_string(uri).map_err(to_string)?;

    let parsed_uri = env
        .call_static_method(
            "android/net/Uri",
            "parse",
            "(Ljava/lang/String;)Landroid/net/Uri;",
            &[JValue::Object(&uri)],
        )
        .and_then(|value| value.l())
        .map_err(to_string)?;

    let intent = env
        .new_object(
            "android/content/Intent",
            "(Ljava/lang/String;Landroid/net/Uri;)V",
            &[JValue::Object(&action), JValue::Object(&parsed_uri)],
        )
        .map_err(to_string)?;

    add_new_task_flag(&mut env, &intent)?;
    start_activity(&mut env, &context, &intent)
}

fn add_new_task_flag(env: &mut AttachGuard<'_>, intent: &JObject<'_>) -> Result<(), String> {
    let flag = env
        .get_static_field("android/content/Intent", "FLAG_ACTIVITY_NEW_TASK", "I")
        .and_then(|value| value.i())
        .map_err(to_string)?;

    env.call_method(
        intent,
        "addFlags",
        "(I)Landroid/content/Intent;",
        &[JValue::Int(flag)],
    )
    .map_err(to_string)?;

    Ok(())
}

fn start_activity(
    env: &mut AttachGuard<'_>,
    context: &JObject<'_>,
    intent: &JObject<'_>,
) -> Result<(), String> {
    env.call_method(
        context,
        "startActivity",
        "(Landroid/content/Intent;)V",
        &[JValue::Object(intent)],
    )
    .map_err(to_string)?;

    Ok(())
}

fn vm() -> Result<&'static JavaVM, String> {
    if let Some(vm) = JVM.get() {
        return Ok(vm);
    }

    let context = ndk_context::android_context();
    let vm = unsafe { JavaVM::from_raw(context.vm() as *mut jni::sys::JavaVM) }
        .map_err(to_string)?;

    let _ = JVM.set(vm);
    JVM.get()
        .ok_or_else(|| "failed to initialize Java VM".to_string())
}

fn env() -> Result<AttachGuard<'static>, String> {
    vm()?.attach_current_thread().map_err(to_string)
}

fn context() -> JObject<'static> {
    let android_context = ndk_context::android_context();
    unsafe { JObject::from_raw(android_context.context() as jni::sys::jobject) }
}

fn to_string(error: impl ToString) -> String {
    error.to_string()
}
