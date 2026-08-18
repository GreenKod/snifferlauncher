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
#[allow(clippy::needless_pass_by_value)]
pub fn launch_action(action: sniffer_core::types::Action) -> Result<(), String> {
    use sniffer_core::types::Action;
    match action {
        Action::OpenSettings => start_action("android.settings.SETTINGS"),
        Action::OpenContacts => start_view_uri("content://contacts/people"),
        Action::OpenCamera => start_action("android.media.action.STILL_IMAGE_CAMERA"),
        Action::LaunchApp { package_name } => launch_app(&package_name),
        Action::RequestDefaultLauncher => request_default_launcher(),
        Action::LoadImage { .. } | Action::FocusTextInput(_) | Action::BlurTextInput => Ok(()),
    }
}

/// Requests default launcher role via RoleManager (Android 10+) or Home Settings intent (Android 9 and below).
pub fn request_default_launcher() -> Result<(), String> {
    use jni::{jni_sig, jni_str, objects::JValue};

    let jvm = vm();

    jvm.attach_current_thread_for_scope::<_, _, jni::errors::Error>(|env: &mut Env| {
        let ctx = context(env);

        let sdk_version = env
            .get_static_field(
                jni_str!("android/os/Build$VERSION"),
                jni_str!("SDK_INT"),
                jni_sig!("I"),
            )?
            .i()?;

        if sdk_version >= 29 {
            let role_service_str = env.new_string("role")?;
            let role_mgr = env
                .call_method(
                    &ctx,
                    jni_str!("getSystemService"),
                    jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"),
                    &[JValue::Object(&role_service_str)],
                )?
                .l()?;

            if !role_mgr.is_null() {
                let role_home_str = env.new_string("android.app.role.HOME")?;
                let intent = env
                    .call_method(
                        &role_mgr,
                        jni_str!("createRequestRoleIntent"),
                        jni_sig!("(Ljava/lang/String;)Landroid/content/Intent;"),
                        &[JValue::Object(&role_home_str)],
                    )?
                    .l()?;

                if !intent.is_null() {
                    add_new_task_flag(env, &intent).map_err(|_| jni::errors::Error::JavaException)?;
                    start_activity(env, &ctx, &intent).map_err(|_| jni::errors::Error::JavaException)?;
                    return Ok(());
                }
            }
        }

        // Fallback for Android 9 and lower or if RoleManager intent creation failed
        let action_str = env.new_string("android.settings.HOME_SETTINGS")?;
        let intent = env.new_object(
            jni_str!("android/content/Intent"),
            jni_sig!("(Ljava/lang/String;)V"),
            &[(&action_str).into()],
        )?;

        add_new_task_flag(env, &intent).map_err(|_| jni::errors::Error::JavaException)?;
        start_activity(env, &ctx, &intent).map_err(|_| jni::errors::Error::JavaException)?;

        Ok(())
    })
    .map_err(|e| e.to_string())
}

/// Launches an Android application using its package name via `getLaunchIntentForPackage`.
pub fn launch_app(package_name: &str) -> Result<(), String> {
    use jni::{jni_sig, jni_str, objects::JValue};

    let jvm = vm();

    jvm.attach_current_thread_for_scope::<_, _, jni::errors::Error>(|env: &mut Env| {
        let ctx = context(env);

        let pm = env
            .call_method(
                &ctx,
                jni_str!("getPackageManager"),
                jni_sig!("()Landroid/content/pm/PackageManager;"),
                &[],
            )?
            .l()?;

        let pkg_string = env.new_string(package_name)?;

        let intent = env
            .call_method(
                &pm,
                jni_str!("getLaunchIntentForPackage"),
                jni_sig!("(Ljava/lang/String;)Landroid/content/Intent;"),
                &[JValue::Object(&pkg_string)],
            )?
            .l()?;

        let intent = if intent.is_null() {
            let _ = env.exception_clear();
            let intent_cls = env.find_class(jni_str!("android/content/Intent"))?;
            let action_main = env
                .get_static_field(
                    &intent_cls,
                    jni_str!("ACTION_MAIN"),
                    jni_sig!("Ljava/lang/String;"),
                )?
                .l()?;
            let cat_launcher = env
                .get_static_field(
                    &intent_cls,
                    jni_str!("CATEGORY_LAUNCHER"),
                    jni_sig!("Ljava/lang/String;"),
                )?
                .l()?;
            let fb_intent = env.new_object(
                &intent_cls,
                jni_sig!("(Ljava/lang/String;)V"),
                &[JValue::Object(&action_main)],
            )?;
            let _ = env.call_method(
                &fb_intent,
                jni_str!("addCategory"),
                jni_sig!("(Ljava/lang/String;)Landroid/content/Intent;"),
                &[JValue::Object(&cat_launcher)],
            )?;
            let _ = env.call_method(
                &fb_intent,
                jni_str!("setPackage"),
                jni_sig!("(Ljava/lang/String;)Landroid/content/Intent;"),
                &[JValue::Object(&pkg_string)],
            )?;

            let resolve_infos = env
                .call_method(
                    &pm,
                    jni_str!("queryIntentActivities"),
                    jni_sig!("(Landroid/content/Intent;I)Ljava/util/List;"),
                    &[JValue::Object(&fb_intent), JValue::Int(0)],
                )?
                .l()?;
            let size = env
                .call_method(&resolve_infos, jni_str!("size"), jni_sig!("()I"), &[])?
                .i()?;

            if size > 0 {
                let res_info = env
                    .call_method(
                        &resolve_infos,
                        jni_str!("get"),
                        jni_sig!("(I)Ljava/lang/Object;"),
                        &[JValue::Int(0)],
                    )?
                    .l()?;
                let act_info = env
                    .get_field(
                        &res_info,
                        jni_str!("activityInfo"),
                        jni_sig!("Landroid/content/pm/ActivityInfo;"),
                    )?
                    .l()?;
                let act_name = env
                    .get_field(
                        &act_info,
                        jni_str!("name"),
                        jni_sig!("Ljava/lang/String;"),
                    )?
                    .l()?;
                let comp_name = env.new_object(
                    jni_str!("android/content/ComponentName"),
                    jni_sig!("(Ljava/lang/String;Ljava/lang/String;)V"),
                    &[JValue::Object(&pkg_string), JValue::Object(&act_name)],
                )?;
                let _ = env.call_method(
                    &fb_intent,
                    jni_str!("setComponent"),
                    jni_sig!("(Landroid/content/ComponentName;)Landroid/content/Intent;"),
                    &[JValue::Object(&comp_name)],
                )?;
                fb_intent
            } else {
                return Err(jni::errors::Error::JavaException);
            }
        } else {
            intent
        };

        sniffer_core::dev_log!("[JNI] Starting activity for package: {package_name}");
        add_new_task_flag(env, &intent).map_err(|e| {
            sniffer_core::dev_err!("[JNI] add_new_task_flag failed: {e}");
            jni::errors::Error::JavaException
        })?;
        start_activity(env, &ctx, &intent).map_err(|e| {
            sniffer_core::dev_err!("[JNI] start_activity failed: {e}");
            jni::errors::Error::JavaException
        })?;

        sniffer_core::dev_log!("[JNI] Successfully started activity for: {package_name}");
        Ok(())
    })
    .map_err(|e| e.to_string())
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


