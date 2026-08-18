use super::{context, vm};
use sniffer_core::dev_log;
use sniffer_core::types::AppInfo;
use jni::errors::Error as JniError;
use jni::{Env, jni_sig, jni_str};
use jni::objects::{JString, JValue};
use obfstr::obfstr;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;

static APP_LIST_CACHE: RwLock<Option<Vec<AppInfo>>> = RwLock::new(None);
static APP_LIST_UPDATED: AtomicBool = AtomicBool::new(false);

pub fn take_app_list_updated() -> bool {
    APP_LIST_UPDATED.swap(false, Ordering::AcqRel)
}

fn get_apps_cache_path() -> Option<String> {
    let jvm = vm();
    jvm.attach_current_thread_for_scope::<_, _, JniError>(|env| {
        let ctx = context(env);
        let files_dir = env.call_method(&ctx, jni_str!("getCacheDir"), jni_sig!("()Ljava/io/File;"), &[])?.l()?;
        let path_obj = env.call_method(&files_dir, jni_str!("getAbsolutePath"), jni_sig!("()Ljava/lang/String;"), &[])?.l()?;
        let path_jstring = env.as_cast::<JString>(&path_obj)?;
        let path_str = path_jstring.try_to_string(env)?;
        Ok(format!("{}/apps_cache.json", path_str))
    }).ok()
}

pub fn init_app_list_cache() {
    std::thread::spawn(|| {
        if let Ok(list) = fetch_application_list_internal() {
            let mut changed = true;
            if let Ok(guard) = APP_LIST_CACHE.read() {
                if let Some(old) = &*guard {
                    if old.len() == list.len() {
                        changed = false;
                    }
                }
            }
            if changed {
                if let Some(path) = get_apps_cache_path() {
                    if let Ok(json) = serde_json::to_string(&list) {
                        let _ = std::fs::write(&path, json);
                    }
                }
                if let Ok(mut guard) = APP_LIST_CACHE.write() {
                    *guard = Some(list);
                }
                APP_LIST_UPDATED.store(true, Ordering::Release);
                sniffer_core::types::UI_VERSION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
    });
}

/// Returns the list of installed application package names.
///
/// # Errors
///
/// Returns an error if JNI calls fail while querying the package manager or
/// converting Java strings to Rust strings.
pub fn get_application_list() -> Result<Vec<AppInfo>, String> {
    if let Ok(guard) = APP_LIST_CACHE.read() {
        if let Some(cached) = &*guard {
            return Ok(cached.clone());
        }
    }
    
    // Try to load from disk synchronously on cold boot
    if let Some(path) = get_apps_cache_path() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(list) = serde_json::from_str::<Vec<AppInfo>>(&content) {
                if let Ok(mut guard) = APP_LIST_CACHE.write() {
                    *guard = Some(list.clone());
                }
                return Ok(list);
            }
        }
    }
    
    Ok(Vec::new())
}

fn fetch_application_list_internal() -> Result<Vec<AppInfo>, String> {
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

/// Opens the Android system settings dialog for setting the Default Home/Launcher app.
pub fn open_default_home_picker() {
    let jvm = vm();
    let _ = jvm.attach_current_thread_for_scope::<_, _, JniError>(|env: &mut Env| {
        let ctx = context(env);
        let intent_cls = env.find_class(jni_str!("android/content/Intent"))?;

        let action_str = env.new_string("android.settings.HOME_SETTINGS")?;
        let intent = env.new_object(
            &intent_cls,
            jni_sig!("(Ljava/lang/String;)V"),
            &[JValue::Object(&action_str)],
        )?;

        let flag = env
            .get_static_field(
                &intent_cls,
                jni_str!("FLAG_ACTIVITY_NEW_TASK"),
                jni_sig!("I"),
            )?
            .i()?;

        let _ = env.call_method(
            &intent,
            jni_str!("addFlags"),
            jni_sig!("(I)Landroid/content/Intent;"),
            &[JValue::Int(flag)],
        );

        let _ = env.call_method(
            &ctx,
            jni_str!("startActivity"),
            jni_sig!("(Landroid/content/Intent;)V"),
            &[JValue::Object(&intent)],
        );

        Ok(())
    });
}


