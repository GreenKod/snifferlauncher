use super::system::{request_default_launcher, start_action, start_view_uri};
use crate::jni::bridge::{add_new_task_flag, context, start_activity, vm};
use jni::objects::JValue;
use jni::{Env, jni_sig, jni_str};
use sniffer_core::types::Action;

#[allow(clippy::needless_pass_by_value)]
pub fn launch_action(action: Action) -> Result<(), String> {
    match action {
        Action::OpenSettings => start_action("android.settings.SETTINGS"),
        Action::OpenContacts => start_view_uri("content://contacts/people"),
        Action::OpenCamera => start_action("android.media.action.STILL_IMAGE_CAMERA"),
        Action::LaunchApp { package_name } => launch_app(&package_name),
        Action::RequestDefaultLauncher => request_default_launcher(),
        Action::LoadImage { .. } | Action::FocusTextInput(_) | Action::BlurTextInput => Ok(()),
    }
}

pub fn launch_app(package_name: &str) -> Result<(), String> {
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
            env.exception_clear();
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
                    .get_field(&act_info, jni_str!("name"), jni_sig!("Ljava/lang/String;"))?
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
