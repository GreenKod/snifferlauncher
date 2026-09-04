use super::{context, vm};
use jni::errors::Error as JniError;
use jni::objects::{JString, JValue};
use jni::{Env, jni_sig, jni_str};
use sniffer_core::types::AppInfo;

use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};

static APP_LIST_CACHE: RwLock<Option<Vec<AppInfo>>> = RwLock::new(None);
static APP_LIST_UPDATED: AtomicBool = AtomicBool::new(false);

pub fn take_app_list_updated() -> bool {
    APP_LIST_UPDATED.swap(false, Ordering::AcqRel)
}

fn get_apps_cache_path() -> Option<String> {
    let jvm = vm();
    jvm.attach_current_thread_for_scope::<_, _, JniError>(|env| {
        let res = (|| -> Result<String, JniError> {
            let ctx = context(env);
            if ctx.is_null() {
                return Err(JniError::NullPtr("context"));
            }
            let files_dir = env
                .call_method(
                    &ctx,
                    jni_str!("getCacheDir"),
                    jni_sig!("()Ljava/io/File;"),
                    &[],
                )?
                .l()?;
            if files_dir.is_null() {
                return Err(JniError::NullPtr("files_dir"));
            }
            let path_obj = env
                .call_method(
                    &files_dir,
                    jni_str!("getAbsolutePath"),
                    jni_sig!("()Ljava/lang/String;"),
                    &[],
                )?
                .l()?;
            let path_jstring = env.as_cast::<JString>(&path_obj)?;
            let path_str = path_jstring.try_to_string(env)?;
            Ok(format!("{path_str}/apps_cache.json"))
        })();

        if res.is_err() {
            env.exception_clear();
        }

        res
    })
    .ok()
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
            let res = (|| -> Result<Vec<AppInfo>, JniError> {
                let ctx = context(env);
                if ctx.is_null() {
                    return Ok(Vec::new());
                }
                let mut local_list = Vec::new();

                let package_manager = env
                    .call_method(
                        &ctx,
                        jni_str!("getPackageManager"),
                        jni_sig!("()Landroid/content/pm/PackageManager;"),
                        &[],
                    )?
                    .l()?;

                if package_manager.is_null() {
                    return Ok(local_list);
                }

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
                let _ = env.call_method(
                    &intent,
                    jni_str!("addCategory"),
                    jni_sig!("(Ljava/lang/String;)Landroid/content/Intent;"),
                    &[JValue::Object(&category_launcher)],
                );
                env.exception_clear();

                let resolve_infos = env
                    .call_method(
                        &package_manager,
                        jni_str!("queryIntentActivities"),
                        jni_sig!("(Landroid/content/Intent;I)Ljava/util/List;"),
                        &[JValue::Object(&intent), JValue::Int(0)],
                    )?
                    .l()?;

                if resolve_infos.is_null() {
                    return Ok(local_list);
                }

                let size = env
                    .call_method(&resolve_infos, jni_str!("size"), jni_sig!("()I"), &[])?
                    .i()?;

                log::info!(
                    "[Apps] Found {} installed activities via queryIntentActivities",
                    size
                );

                for i in 0..size {
                    let item_res: Result<(String, String), JniError> = (|| {
                        let resolve_info = env
                            .call_method(
                                &resolve_infos,
                                jni_str!("get"),
                                jni_sig!("(I)Ljava/lang/Object;"),
                                &[JValue::Int(i)],
                            )?
                            .l()?;

                        if resolve_info.is_null() {
                            return Err(JniError::NullPtr("resolve_info"));
                        }

                        let activity_info = env
                            .get_field(
                                &resolve_info,
                                jni_str!("activityInfo"),
                                jni_sig!("Landroid/content/pm/ActivityInfo;"),
                            )?
                            .l()?;

                        if activity_info.is_null() {
                            return Err(JniError::NullPtr("activityInfo"));
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
                                jni_sig!(
                                    "(Landroid/content/pm/PackageManager;)Ljava/lang/CharSequence;"
                                ),
                                &[JValue::Object(&package_manager)],
                            )?
                            .l()?;

                        let package_name_jstring = env.as_cast::<JString>(&package_name_obj)?;
                        let package_name_str = package_name_jstring.try_to_string(env)?;

                        let app_name = if !label_char_seq.is_null() {
                            let label_jstring = env
                                .call_method(
                                    &label_char_seq,
                                    jni_str!("toString"),
                                    jni_sig!("()Ljava/lang/String;"),
                                    &[],
                                )?
                                .l()?;
                            let label_jstring = env.as_cast::<JString>(&label_jstring)?;
                            label_jstring.try_to_string(env)?
                        } else {
                            package_name_str.clone()
                        };

                        Ok((app_name, package_name_str))
                    })();

                    match item_res {
                        Ok((app_name, package_name_str)) => {
                            log::info!(
                                "[Apps] Loaded app #{}: {} ({})",
                                i,
                                app_name,
                                package_name_str
                            );
                            local_list.push(AppInfo::new(app_name, package_name_str));
                        }
                        Err(e) => {
                            log::warn!("[Apps] Error loading app #{}: {:?}", i, e);
                            env.exception_clear();
                        }
                    }
                }

                Ok(local_list)
            })();

            if res.is_err() {
                env.exception_clear();
            }

            res
        })
        .map_err(|e: JniError| e.to_string())?;

    log::info!("[Apps] Total apps found: {}", app_list.len());

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
                permission_array.set_element(env, index, &j_permission)?;
            }

            let activity_class = env.find_class(jni_str!("android/app/Activity"))?;
            let _res = env.call_static_method(
                activity_class,
                jni_str!("requestPermissions"),
                jni_sig!("(Landroid/app/Activity;[Ljava/lang/String;I)V"),
                &[],
            )?;

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
