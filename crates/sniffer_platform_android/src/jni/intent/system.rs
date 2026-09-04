use crate::jni::bridge::{add_new_task_flag, context, start_activity, vm};
use android_activity::AndroidApp;
use jni::objects::JValue;
use jni::{Env, jni_sig, jni_str};

pub fn get_safe_area(_app: &AndroidApp) -> Option<(i32, i32)> {
    let jvm = vm();

    jvm.attach_current_thread_for_scope::<_, _, jni::errors::Error>(|env: &mut Env| {
        let res = (|| -> Result<Option<(i32, i32)>, jni::errors::Error> {
            let activity = context(env);
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

            if window.is_null() {
                return Ok(None);
            }

            let decor_view = env
                .call_method(
                    &window,
                    jni_str!("getDecorView"),
                    jni_sig!("()Landroid/view/View;"),
                    &[],
                )?
                .l()?;

            if decor_view.is_null() {
                return Ok(None);
            }

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
        })();

        if res.is_err() {
            env.exception_clear();
        }

        res
    })
    .ok()
    .flatten()
}

pub fn request_default_launcher() -> Result<(), String> {
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
                    add_new_task_flag(env, &intent)
                        .map_err(|_| jni::errors::Error::JavaException)?;
                    start_activity(env, &ctx, &intent)
                        .map_err(|_| jni::errors::Error::JavaException)?;
                    return Ok(());
                }
            }
        }

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

#[allow(dead_code)]
pub fn start_action(action: &str) -> Result<(), String> {
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

#[allow(dead_code)]
pub fn start_view_uri(uri: &str) -> Result<(), String> {
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
