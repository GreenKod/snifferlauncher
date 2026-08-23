use crate::dev_err;
use crate::js::permission_manager::permission_granted;
use obfstr::obfstr;
use rquickjs::{Ctx, Function, Object};
use std::sync::{Arc, Mutex};

#[allow(clippy::too_many_lines)]
pub fn register_app_launch_bindings<'js>(
    ctx: &Ctx<'js>,
    globals: &Object<'js>,
    action_queue: Arc<Mutex<Vec<sniffer_core::types::Action>>>,
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

        let result = super::super::host_bridge::get_application_list();

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
        if let Err(e) = super::super::host_bridge::launch_app(&package_name) {
            crate::dev_err!("{}: {e}", obfstr!("Direct host_launch_app failed"));
        }
        if let Ok(mut q) = aq_launch.lock() {
            q.push(sniffer_core::types::Action::LaunchApp { package_name });
        }
    })
    .unwrap();
    globals
        .set(obfstr!("host_launch_app"), launch_app_func)
        .unwrap();

    // host_request_default_launcher
    let aq_req_home = action_queue.clone();
    let req_home_func = Function::new(ctx.clone(), move || {
        let _ = super::super::host_bridge::open_default_home_picker();
        if let Ok(mut q) = aq_req_home.lock() {
            q.push(sniffer_core::types::Action::RequestDefaultLauncher);
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
                dev_err!(
                    "{}",
                    obfstr!("Plugin requested permissions it did not declare or are invalid.")
                );
                return "[]".to_string();
            }

            super::super::host_bridge::request_permissions(&valid_permissions);
            let granted: Result<Vec<String>, String> = Ok(valid_permissions);

            match granted {
                Ok(granted) => {
                    if let Ok(mut lock) = granted_permissions_clone2.lock() {
                        for perm in &granted {
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
        .set(
            obfstr!("host_request_permissions"),
            request_permissions_func,
        )
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
            q.push(sniffer_core::types::Action::LoadImage { id, src });
        }
    })
    .unwrap();
    globals
        .set(obfstr!("host_create_image"), create_image_func)
        .unwrap();

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
            q.push(sniffer_core::types::Action::FocusTextInput(id));
        }
    })
    .unwrap();
    globals
        .set(obfstr!("host_focus_input"), focus_input_func)
        .unwrap();

    // host_blur_input
    let aq_blur = action_queue;
    let plugin_permissions_blur = plugin_permissions_vec;
    let granted_permissions_blur = granted_permissions;
    let blur_input_func = Function::new(ctx.clone(), move || {
        if !permission_granted(
            obfstr!("plugin.permission.INPUT"),
            &plugin_permissions_blur,
            &granted_permissions_blur,
        ) {
            return;
        }

        if let Ok(mut q) = aq_blur.lock() {
            q.push(sniffer_core::types::Action::BlurTextInput);
        }
    })
    .unwrap();
    globals
        .set(obfstr!("host_blur_input"), blur_input_func)
        .unwrap();
}
