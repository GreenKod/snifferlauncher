pub mod apps;
pub mod ipc;

use crate::registry::{ApiMap, BroadcastQueue};
use crossbeam_channel::Sender;
use rquickjs::{Ctx, Object};
use std::sync::{Arc, Mutex};

pub fn register_app_bindings<'js>(
    ctx: &Ctx<'js>,
    globals: &Object<'js>,
    plugin_id: String,
    is_master: bool,
    msg_tx: Sender<crate::js::plugin::PluginMsg>,
    action_queue: Arc<Mutex<Vec<sniffer_core::types::Action>>>,
    api_map: ApiMap,
    broadcast_queue: BroadcastQueue,
    plugin_permissions: &[String],
    granted_permissions: Arc<Mutex<Vec<String>>>,
) {
    apps::register_app_launch_bindings(
        ctx,
        globals,
        plugin_id.clone(),
        is_master,
        action_queue,
        plugin_permissions,
        granted_permissions.clone(),
    );

    ipc::register_ipc_bindings(
        ctx,
        globals,
        plugin_id,
        msg_tx,
        api_map,
        broadcast_queue,
        plugin_permissions,
        granted_permissions,
    );
}
