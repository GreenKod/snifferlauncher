use crate::js::permission_manager::permission_granted;
use crate::registry::{ApiEntry, ApiMap, BroadcastQueue};
use crossbeam_channel::Sender;
use obfstr::obfstr;
use rquickjs::{Ctx, Function, Object};
use std::sync::{Arc, Mutex};

pub fn register_ipc_bindings<'js>(
    ctx: &Ctx<'js>,
    globals: &Object<'js>,
    plugin_id: String,
    msg_tx: Sender<crate::js::plugin::PluginMsg>,
    api_map: ApiMap,
    broadcast_queue: BroadcastQueue,
    plugin_permissions: &[String],
    granted_permissions: Arc<Mutex<Vec<String>>>,
) {
    let api_map_register = api_map.clone();
    let owning_plugin_id = plugin_id.clone();
    let msg_tx_api = msg_tx;

    let plugin_permissions_ipc = plugin_permissions.to_vec();
    let granted_permissions_ipc = granted_permissions;
    let register_api_func = Function::new(ctx.clone(), move |name: String| {
        if !permission_granted(
            obfstr!("plugin.permission.IPC"),
            &plugin_permissions_ipc,
            &granted_permissions_ipc,
        ) {
            return;
        }

        let msg_tx_clone = msg_tx_api.clone();
        let api_name_clone = name.clone();
        let owning_id = owning_plugin_id.clone();

        let callback: Arc<dyn Fn(String) -> Option<String> + Send + Sync> =
            Arc::new(move |payload_json: String| {
                let (resp_tx, resp_rx) = crossbeam_channel::bounded(1);
                if msg_tx_clone
                    .send(crate::js::plugin::PluginMsg::ApiCall {
                        name: api_name_clone.clone(),
                        payload: payload_json,
                        responder: resp_tx,
                    })
                    .is_err()
                {
                    return None;
                }
                resp_rx
                    .recv_timeout(std::time::Duration::from_millis(1500))
                    .unwrap_or(None)
            });

        if let Ok(mut map) = api_map_register.lock() {
            if map.contains_key(&name) {
                crate::logger::warn(
                    &owning_id,
                    &format!("API '{name}' is already registered, overwriting"),
                );
            }
            map.insert(
                name.clone(),
                ApiEntry {
                    plugin_id: owning_id.clone(),
                    callback,
                },
            );
            crate::logger::info(&owning_id, &format!("registered API '{name}'"));
        }
    })
    .unwrap();
    globals
        .set(obfstr!("host_register_api"), register_api_func)
        .unwrap();

    // host_call_api
    let api_map_call = api_map;
    let calling_plugin_id = plugin_id.clone();

    let call_api_func = Function::new(
        ctx.clone(),
        move |name: String, payload_json: String| -> Option<String> {
            let map = match api_map_call.lock() {
                Ok(m) => m,
                Err(_) => {
                    crate::logger::error(
                        &calling_plugin_id,
                        &format!("failed to lock ApiMap for call to '{name}'"),
                    );
                    return None;
                }
            };

            if let Some(entry) = map.get(&name) {
                crate::logger::info(
                    &calling_plugin_id,
                    &format!(
                        "calling API '{name}' (owned by '{}')",
                        crate::logger::clean_plugin_id(&entry.plugin_id)
                    ),
                );
                let callback = Arc::clone(&entry.callback);
                drop(map);
                callback(payload_json)
            } else {
                crate::logger::error(
                    &calling_plugin_id,
                    &format!("tried to call unknown API '{name}'"),
                );
                None
            }
        },
    )
    .unwrap();
    globals
        .set(obfstr!("host_call_api"), call_api_func)
        .unwrap();

    // host_broadcast
    let bq = broadcast_queue;
    let broadcast_plugin_id = plugin_id;
    let broadcast_func =
        Function::new(ctx.clone(), move |channel: String, payload_json: String| {
            crate::logger::info(
                &broadcast_plugin_id,
                &format!("broadcasting on channel '{channel}'"),
            );
            if let Ok(mut q) = bq.lock() {
                q.push((channel, payload_json));
            }
        })
        .unwrap();
    globals
        .set(obfstr!("host_broadcast"), broadcast_func)
        .unwrap();
}
