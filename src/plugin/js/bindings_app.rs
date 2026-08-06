use crate::plugin::js::permission_manager::permission_granted;
use crate::plugin::registry::{ApiEntry, ApiMap, BroadcastQueue};
use crate::{dev_err, dev_log};
use obfstr::obfstr;
use rquickjs::{Ctx, Function, Object};
use std::sync::{Arc, Mutex};

pub struct SafeContext(pub(crate) rquickjs::Context);

#[allow(clippy::non_send_fields_in_send_ty)]
unsafe impl Send for SafeContext {}
unsafe impl Sync for SafeContext {}

#[allow(clippy::too_many_lines)]
pub fn register_app_bindings<'js>(
    ctx: &Ctx<'js>,
    globals: &Object<'js>,
    plugin_id: String,
    context: rquickjs::Context,
    action_queue: Arc<Mutex<Vec<crate::core::types::Action>>>,
    api_map: ApiMap,
    broadcast_queue: BroadcastQueue,
    plugin_permissions: &[String],
    granted_permissions: Arc<Mutex<Vec<String>>>,
) {
    let plugin_permissions_vec = plugin_permissions.to_vec();

    // host_get_application_list
    let plugin_permissions_app_list = plugin_permissions_vec.clone();
    let granted_permissions_app_list = Arc::clone(&granted_permissions);
    let get_app_list_func = Function::new(ctx.clone(), move || -> String {
        if !permission_granted(
            obfstr!("android.permission.QUERY_ALL_PACKAGES"),
            &plugin_permissions_app_list,
            &granted_permissions_app_list,
        ) {
            return "[]".to_string();
        }

        let result = {
            #[cfg(target_os = "android")]
            {
                crate::platform::android::jni::bridge::get_application_list()
            }

            #[cfg(not(target_os = "android"))]
            {
                crate::platform::desktop::get_application_list()
            }
        };

        match result {
            Ok(apps) => serde_json::to_string(&apps).unwrap_or_else(|_| "[]".to_string()),
            Err(e) => {
                dev_err!("{}: {e}", obfstr!("Failed to get application list"));
                "[]".to_string()
            }
        }
    })
    .unwrap();
    globals
        .set(obfstr!("host_get_application_list"), get_app_list_func)
        .unwrap();

    // host_launch_app
    let aq_launch = action_queue.clone();
    let launch_app_func = Function::new(ctx.clone(), move |package_name: String| {
        if let Ok(mut q) = aq_launch.lock() {
            q.push(crate::core::types::Action::LaunchApp { package_name });
        }
    })
    .unwrap();
    globals.set(obfstr!("host_launch_app"), launch_app_func).unwrap();

    // host_request_default_launcher
    let aq_req_home = action_queue.clone();
    let req_home_func = Function::new(ctx.clone(), move || {
        if let Ok(mut q) = aq_req_home.lock() {
            q.push(crate::core::types::Action::RequestDefaultLauncher);
        }
    })
    .unwrap();
    globals
        .set(obfstr!("host_request_default_launcher"), req_home_func)
        .unwrap();

    // host_has_permission
    let granted_permissions_has = Arc::clone(&granted_permissions);
    let has_permission_func = Function::new(ctx.clone(), move |permission: String| -> bool {
        if let Ok(lock) = granted_permissions_has.lock() {
            lock.contains(&permission)
        } else {
            false
        }
    })
    .unwrap();
    globals
        .set(obfstr!("host_has_permission"), has_permission_func)
        .unwrap();

    // host_request_permissions
    let plugin_permissions_clone2 = plugin_permissions_vec.clone();
    let granted_permissions_clone2 = granted_permissions.clone();
    let request_permissions_func =
        Function::new(ctx.clone(), move |permissions: Vec<String>| -> String {
            let valid_permissions: Vec<String> = permissions
                .into_iter()
                .filter(|p| plugin_permissions_clone2.contains(p))
                .collect();

            if valid_permissions.is_empty() {
                dev_err!("{}", obfstr!("Plugin requested permissions it did not declare or are invalid."));
                return "[]".to_string();
            }

            #[cfg(target_os = "android")]
            let granted =
                crate::platform::android::jni::bridge::request_permissions(&valid_permissions);

            #[cfg(not(target_os = "android"))]
            let granted: Result<Vec<String>, String> = Ok(valid_permissions.clone());

            match granted {
                Ok(granted) => {
                    if let Ok(mut lock) = granted_permissions_clone2.lock() {
                        for perm in granted.iter() {
                            if !lock.contains(perm) {
                                lock.push(perm.clone());
                            }
                        }
                    }
                    serde_json::to_string(&granted).unwrap_or_else(|_| "[]".to_string())
                }
                Err(e) => {
                    dev_err!("{}: {e}", obfstr!("Failed to request permissions"));
                    "[]".to_string()
                }
            }
        })
        .unwrap();
    globals
        .set(obfstr!("host_request_permissions"), request_permissions_func)
        .unwrap();

    // host_create_image
    let aq_img = action_queue.clone();
    let plugin_permissions_image = plugin_permissions_vec.clone();
    let granted_permissions_image = granted_permissions.clone();
    let create_image_func = Function::new(ctx.clone(), move |id: String, src: String| {
        if !permission_granted(
            obfstr!("plugin.permission.IMAGE"),
            &plugin_permissions_image,
            &granted_permissions_image,
        ) {
            return;
        }

        if let Ok(mut q) = aq_img.lock() {
            q.push(crate::core::types::Action::LoadImage { id, src });
        }
    })
    .unwrap();
    globals.set(obfstr!("host_create_image"), create_image_func).unwrap();

    // host_focus_input
    let aq_focus = action_queue.clone();
    let plugin_permissions_focus = plugin_permissions_vec.clone();
    let granted_permissions_focus = granted_permissions.clone();
    let focus_input_func = Function::new(ctx.clone(), move |id: String| {
        if !permission_granted(
            obfstr!("plugin.permission.INPUT"),
            &plugin_permissions_focus,
            &granted_permissions_focus,
        ) {
            return;
        }

        if let Ok(mut q) = aq_focus.lock() {
            q.push(crate::core::types::Action::FocusTextInput(id));
        }
    })
    .unwrap();
    globals.set(obfstr!("host_focus_input"), focus_input_func).unwrap();

    // host_blur_input
    let aq_blur = action_queue;
    let plugin_permissions_blur = plugin_permissions_vec.clone();
    let granted_permissions_blur = granted_permissions.clone();
    let blur_input_func = Function::new(ctx.clone(), move || {
        if !permission_granted(
            obfstr!("plugin.permission.INPUT"),
            &plugin_permissions_blur,
            &granted_permissions_blur,
        ) {
            return;
        }

        if let Ok(mut q) = aq_blur.lock() {
            q.push(crate::core::types::Action::BlurTextInput);
        }
    })
    .unwrap();
    globals.set(obfstr!("host_blur_input"), blur_input_func).unwrap();

    // Inter-Plugin API host functions
    let safe_ctx = SafeContext(context);
    let safe_ctx = Arc::new(safe_ctx);
    let api_map_register = api_map.clone();
    let owning_plugin_id = plugin_id.clone();

    let plugin_permissions_ipc = plugin_permissions_vec;
    let granted_permissions_ipc = granted_permissions;
    let register_api_func = Function::new(ctx.clone(), move |name: String| {
        if !permission_granted(
            obfstr!("plugin.permission.IPC"),
            &plugin_permissions_ipc,
            &granted_permissions_ipc,
        ) {
            return;
        }

        let ctx_clone = Arc::clone(&safe_ctx);
        let api_name_clone = name.clone();
        let owning_id = owning_plugin_id.clone();

        let callback: Arc<dyn Fn(String) -> Option<String> + Send + Sync> =
            Arc::new(move |payload_json: String| {
                let mut result: Option<String> = None;
                ctx_clone.0.with(|ctx| {
                    if let Ok(handler) =
                        ctx.globals().get::<_, rquickjs::Function>(obfstr!("_handleApiCall"))
                        && let Ok(ret) = handler
                            .call::<_, rquickjs::Value>((api_name_clone.clone(), payload_json))
                        && ret.is_string()
                    {
                        result = ret.as_string().and_then(|s| s.to_string().ok());
                    }
                });
                result
            });

        if let Ok(mut map) = api_map_register.lock() {
            if map.contains_key(&name) {
                dev_log!(
                    "{} '{}' {} '{}'.",
                    obfstr!("[IPC] Warning: API"),
                    name,
                    obfstr!("is already registered. Overwriting with plugin"),
                    owning_id
                );
            }
            map.insert(
                name.clone(),
                ApiEntry {
                    plugin_id: owning_id.clone(),
                    callback,
                },
            );
            dev_log!("{} '{}' {} '{}'.", obfstr!("[IPC] Plugin"), owning_id, obfstr!("registered API"), name);
        }
    })
    .unwrap();
    globals.set(obfstr!("host_register_api"), register_api_func).unwrap();

    // host_call_api
    let api_map_call = api_map;
    let calling_plugin_id = plugin_id.clone();

    let call_api_func = Function::new(
        ctx.clone(),
        move |name: String, payload_json: String| -> Option<String> {
            let map = match api_map_call.lock() {
                Ok(m) => m,
                Err(_) => {
                    dev_err!("{} '{name}'.", obfstr!("[IPC] Failed to lock ApiMap for call to"));
                    return None;
                }
            };

            if let Some(entry) = map.get(&name) {
                dev_log!(
                    "{} '{}' {} '{}' ({} '{}').",
                    obfstr!("[IPC] Plugin"),
                    calling_plugin_id,
                    obfstr!("calling API"),
                    name,
                    obfstr!("owned by"),
                    entry.plugin_id
                );
                let callback = Arc::clone(&entry.callback);
                drop(map);
                callback(payload_json)
            } else {
                dev_err!(
                    "{} '{}' {} '{}'.",
                    obfstr!("[IPC] Plugin"),
                    calling_plugin_id,
                    obfstr!("tried to call unknown API"),
                    name
                );
                None
            }
        },
    )
    .unwrap();
    globals.set(obfstr!("host_call_api"), call_api_func).unwrap();

    // host_broadcast
    let bq = broadcast_queue;
    let broadcast_plugin_id = plugin_id;
    let broadcast_func =
        Function::new(ctx.clone(), move |channel: String, payload_json: String| {
            dev_log!(
                "{} '{}' {} '{}'.",
                obfstr!("[IPC] Plugin"),
                broadcast_plugin_id,
                obfstr!("broadcasting on channel"),
                channel
            );
            if let Ok(mut q) = bq.lock() {
                q.push((channel, payload_json));
            }
        })
        .unwrap();
    globals.set(obfstr!("host_broadcast"), broadcast_func).unwrap();
}
