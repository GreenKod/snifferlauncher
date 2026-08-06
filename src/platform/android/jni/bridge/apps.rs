use super::{context, vm};
use crate::dev_log;
use crate::core::types::AppInfo;
use jni::errors::Error as JniError;
use jni::{Env, jni_sig, jni_str};
use jni::objects::{JString, JValue};
use obfstr::obfstr;

/// Returns the list of installed application package names.
///
/// # Errors
///
/// Returns an error if JNI calls fail while querying the package manager or
/// converting Java strings to Rust strings.
pub fn get_application_list() -> Result<Vec<AppInfo>, String> {
    let jvm = vm();

    let app_list = jvm
        .attach_current_thread_for_scope::<_, _, JniError>(|env: &mut Env| {
            let ctx = context(env);
            let mut local_list = Vec::new();

            let package_manager = env
                .call_method(
                    &ctx,
                    jni_str!("getPackageManager"),
                    jni_sig!("()Landroid/content/pm/PackageManager;"),
                    &[],
                )?
                .l()?;

            let intent_cls = env.find_class(jni_str!("android/content/Intent"))?;
            let action_main = env
                .get_static_field(
                    &intent_cls,
                    jni_str!("ACTION_MAIN"),
                    jni_sig!("Ljava/lang/String;"),
                )?
                .l()?;
            let category_launcher = env
                .get_static_field(
                    &intent_cls,
                    jni_str!("CATEGORY_LAUNCHER"),
                    jni_sig!("Ljava/lang/String;"),
                )?
                .l()?;

            let intent = env.new_object(
                &intent_cls,
                jni_sig!("(Ljava/lang/String;)V"),
                &[JValue::Object(&action_main)],
            )?;
            env.call_method(
                &intent,
                jni_str!("addCategory"),
                jni_sig!("(Ljava/lang/String;)Landroid/content/Intent;"),
                &[JValue::Object(&category_launcher)],
            )?;

            let resolve_infos = env
                .call_method(
                    &package_manager,
                    jni_str!("queryIntentActivities"),
                    jni_sig!("(Landroid/content/Intent;I)Ljava/util/List;"),
                    &[JValue::Object(&intent), JValue::Int(0)],
                )?
                .l()?;

            let size = env
                .call_method(&resolve_infos, jni_str!("size"), jni_sig!("()I"), &[])?
                .i()?;

            for i in 0..size {
                let resolve_info = env
                    .call_method(
                        &resolve_infos,
                        jni_str!("get"),
                        jni_sig!("(I)Ljava/lang/Object;"),
                        &[JValue::Int(i)],
                    )?
                    .l()?;

                let activity_info = env
                    .get_field(
                        &resolve_info,
                        jni_str!("activityInfo"),
                        jni_sig!("Landroid/content/pm/ActivityInfo;"),
                    )?
                    .l()?;

                if activity_info.is_null() {
                    continue;
                }

                let package_name_obj = env
                    .get_field(
                        &activity_info,
                        jni_str!("packageName"),
                        jni_sig!("Ljava/lang/String;"),
                    )?
                    .l()?;

                let label_char_seq = env
                    .call_method(
                        &resolve_info,
                        jni_str!("loadLabel"),
                        jni_sig!("(Landroid/content/pm/PackageManager;)Ljava/lang/CharSequence;"),
                        &[JValue::Object(&package_manager)],
                    )?
                    .l()?;

                let label_jstring = env
                    .call_method(
                        &label_char_seq,
                        jni_str!("toString"),
                        jni_sig!("()Ljava/lang/String;"),
                        &[],
                    )?
                    .l()?;

                let label_jstring = env.as_cast::<JString>(&label_jstring)?;
                let app_name = label_jstring.try_to_string(env)?;

                let package_name_jstring = env.as_cast::<JString>(&package_name_obj)?;
                let package_name_str = package_name_jstring.try_to_string(env)?;

                local_list.push(AppInfo::new(app_name, package_name_str));
            }

            Ok(local_list)
        })
        .map_err(|e: JniError| e.to_string())?;

    if cfg!(debug_assertions) {
        dev_log!(
            "{} {}",
            obfstr!("[DEBUG] Number of apps to display in the launcher:"),
            app_list.len()
        );
        for app in &app_list {
            dev_log!("{} - {} ({})", obfstr!("[DEBUG]"), app.name, app.package_name);
        }
        for app in AppInfo::search_by_name(&app_list, obfstr!("sett")) {
            dev_log!(
                "{} \n - {} ({})",
                obfstr!("[DEBUG] Search result:"),
                app.name, app.package_name
            );
        }
    }

    Ok(app_list)
}

/// Requests the provided Android permissions via the current activity.
pub fn request_permissions(permissions: &[String]) -> Result<Vec<String>, String> {
    if permissions.is_empty() {
        return Ok(Vec::new());
    }

    let jvm = vm();
    let granted = jvm
        .attach_current_thread_for_scope::<_, _, JniError>(|env: &mut Env| {
            let _ctx = context(env);
            let empty_str = env.new_string("")?;
            let string_class = env.find_class(jni_str!("java/lang/String"))?;
            let permission_array = env.new_object_array(
                permissions.len().try_into().unwrap_or(0),
                string_class,
                &empty_str,
            )?;

            for (index, permission) in permissions.iter().enumerate() {
                let j_permission = env.new_string(permission)?;
                permission_array.set_element(
                    env,
                    index.try_into().unwrap_or(0),
                    &j_permission,
                )?;
            }

            let activity_class = env.find_class(jni_str!("android/app/Activity"))?;
            let request_permissions_method = env.call_static_method(
                activity_class,
                jni_str!("requestPermissions"),
                jni_sig!("(Landroid/app/Activity;[Ljava/lang/String;I)V"),
                &[],
            )?;

            let _ = request_permissions_method;

            Ok(permissions.to_vec())
        })
        .map_err(|e: JniError| e.to_string())?;

    Ok(granted)
}
