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

            let mut top = 0i32;
            let mut bottom = 0i32;

            let window = env
                .call_method(
                    &activity,
                    jni_str!("getWindow"),
                    jni_sig!("()Landroid/view/Window;"),
                    &[],
                )?
                .l()?;

            if !window.is_null() {
                let decor_view = env
                    .call_method(
                        &window,
                        jni_str!("getDecorView"),
                        jni_sig!("()Landroid/view/View;"),
                        &[],
                    )?
                    .l()?;

                if !decor_view.is_null() {
                    if let Ok(insets_val) = env.call_method(
                        &decor_view,
                        jni_str!("getRootWindowInsets"),
                        jni_sig!("()Landroid/view/WindowInsets;"),
                        &[],
                    ) {
                        let insets = insets_val.l()?;
                        if !insets.is_null() {
                            // API 30+: insets.getInsets(135 = systemBars | displayCutout)
                            if let Ok(insets_obj_val) = env.call_method(
                                &insets,
                                jni_str!("getInsets"),
                                jni_sig!("(I)Landroid/graphics/Insets;"),
                                &[JValue::Int(135i32)],
                            ) {
                                let insets_obj = insets_obj_val.l()?;
                                if !insets_obj.is_null() {
                                    if let Ok(t) = env
                                        .get_field(&insets_obj, jni_str!("top"), jni_sig!("I"))
                                        .and_then(|v| v.i())
                                    {
                                        top = t;
                                    }
                                    if let Ok(b) = env
                                        .get_field(&insets_obj, jni_str!("bottom"), jni_sig!("I"))
                                        .and_then(|v| v.i())
                                    {
                                        bottom = b;
                                    }
                                }
                            }
                            env.exception_clear();

                            if top == 0 {
                                if let Ok(t) = env
                                    .call_method(
                                        &insets,
                                        jni_str!("getSystemWindowInsetTop"),
                                        jni_sig!("()I"),
                                        &[],
                                    )
                                    .and_then(|v| v.i())
                                {
                                    top = t;
                                }
                                env.exception_clear();
                            }
                            if bottom == 0 {
                                if let Ok(b) = env
                                    .call_method(
                                        &insets,
                                        jni_str!("getSystemWindowInsetBottom"),
                                        jni_sig!("()I"),
                                        &[],
                                    )
                                    .and_then(|v| v.i())
                                {
                                    bottom = b;
                                }
                                env.exception_clear();
                            }
                        }
                    }
                }
            }

            // Fallback to system framework resources if status_bar_height or navigation_bar_height is missing
            if top == 0 || bottom == 0 {
                if let Ok(res_val) = env.call_method(
                    &activity,
                    jni_str!("getResources"),
                    jni_sig!("()Landroid/content/res/Resources;"),
                    &[],
                ) {
                    let res = res_val.l()?;
                    if !res.is_null() {
                        if top == 0 {
                            let name = env.new_string("status_bar_height")?;
                            let dimen = env.new_string("dimen")?;
                            let android = env.new_string("android")?;
                            if let Ok(id) = env
                                .call_method(
                                    &res,
                                    jni_str!("getIdentifier"),
                                    jni_sig!(
                                        "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I"
                                    ),
                                    &[
                                        JValue::Object(&name),
                                        JValue::Object(&dimen),
                                        JValue::Object(&android),
                                    ],
                                )
                                .and_then(|v| v.i())
                            {
                                if id > 0 {
                                    if let Ok(val) = env
                                        .call_method(
                                            &res,
                                            jni_str!("getDimensionPixelSize"),
                                            jni_sig!("(I)I"),
                                            &[JValue::Int(id)],
                                        )
                                        .and_then(|v| v.i())
                                    {
                                        top = val;
                                    }
                                }
                            }
                            env.exception_clear();
                        }
                        if bottom == 0 {
                            let name = env.new_string("navigation_bar_height")?;
                            let dimen = env.new_string("dimen")?;
                            let android = env.new_string("android")?;
                            if let Ok(id) = env
                                .call_method(
                                    &res,
                                    jni_str!("getIdentifier"),
                                    jni_sig!(
                                        "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I"
                                    ),
                                    &[
                                        JValue::Object(&name),
                                        JValue::Object(&dimen),
                                        JValue::Object(&android),
                                    ],
                                )
                                .and_then(|v| v.i())
                            {
                                if id > 0 {
                                    if let Ok(val) = env
                                        .call_method(
                                            &res,
                                            jni_str!("getDimensionPixelSize"),
                                            jni_sig!("(I)I"),
                                            &[JValue::Int(id)],
                                        )
                                        .and_then(|v| v.i())
                                    {
                                        bottom = val;
                                    }
                                }
                            }
                            env.exception_clear();
                        }
                    }
                }
            }

            if top > 0 || bottom > 0 {
                Ok(Some((top, bottom)))
            } else {
                Ok(None)
            }
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
