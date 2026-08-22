pub mod bindings_app;
pub mod bindings_pkg;
pub mod bindings_ui;
pub mod bindings_vault;
pub mod engine;
pub mod host_bridge;
pub mod permission_manager;
pub mod plugin;

use crate::dev_log;
use crate::registry::{ApiMap, BroadcastQueue};
use crossbeam_channel::Sender;
use obfstr::obfstr;
use rquickjs::Function;
use sniffer_core::types::Element;
use sniffer_core::vault::DataVault;
use std::sync::{Arc, Mutex, RwLock};

pub use plugin::{JsPlugin, JsPluginConfig, PluginMsg};

/// Configuration bundle passed to `register_host_api`.
pub struct HostApiConfig {
    pub plugin_id: String,
    pub msg_tx: Sender<PluginMsg>,
    pub vault: Arc<DataVault>,
    pub ui_tree: Arc<Mutex<Option<Element>>>,
    pub action_queue: Arc<Mutex<Vec<sniffer_core::types::Action>>>,
    pub api_map: ApiMap,
    pub broadcast_queue: BroadcastQueue,
    pub plugin_permissions: Vec<String>,
    pub granted_permissions: Arc<Mutex<Vec<String>>>,
    pub default_settings: serde_json::Value,
    pub cache_path: Option<std::path::PathBuf>,
    pub pkg_registry: Option<Arc<RwLock<sniffer_pkg::PackageRegistry>>>,
}

/// Inject all host-provided global functions into a QuickJS context.
pub fn register_host_api(ctx: &rquickjs::Ctx, cfg: HostApiConfig) {
    let HostApiConfig {
        plugin_id,
        msg_tx,
        vault,
        ui_tree,
        action_queue,
        api_map,
        broadcast_queue,
        plugin_permissions,
        granted_permissions,
        default_settings,
        cache_path,
        pkg_registry,
    } = cfg;

    let globals = ctx.globals();

    // host_get_default_settings
    let default_settings_json =
        serde_json::to_string(&default_settings).unwrap_or_else(|_| "{}".to_string());
    let get_default_settings_func = Function::new(ctx.clone(), move || -> String {
        default_settings_json.clone()
    })
    .unwrap();
    globals
        .set(
            obfstr!("host_get_default_settings"),
            get_default_settings_func,
        )
        .unwrap();

    // host_log
    let log_func = Function::new(ctx.clone(), |msg: String| {
        dev_log!("{}: {msg}", obfstr!("JS Log"));
    })
    .unwrap();
    globals.set(obfstr!("host_log"), log_func).unwrap();

    // host_hash
    let hash_func = Function::new(ctx.clone(), |s: String| -> String {
        sniffer_core::ui::widget::fnv1a(s.as_bytes()).to_string()
    })
    .unwrap();
    globals.set(obfstr!("host_hash"), hash_func).unwrap();

    // host_screen_width
    let get_width_func = Function::new(ctx.clone(), || -> f32 {
        f32::from_bits(sniffer_core::types::SCREEN_WIDTH.load(std::sync::atomic::Ordering::Relaxed))
    })
    .unwrap();
    globals
        .set(obfstr!("host_screen_width"), get_width_func)
        .unwrap();

    // host_screen_height
    let get_height_func = Function::new(ctx.clone(), || -> f32 {
        f32::from_bits(
            sniffer_core::types::SCREEN_HEIGHT.load(std::sync::atomic::Ordering::Relaxed),
        )
    })
    .unwrap();
    globals
        .set(obfstr!("host_screen_height"), get_height_func)
        .unwrap();

    // host_get_local_time
    let get_local_time_func = Function::new(ctx.clone(), || -> String {
        let now = chrono::Local::now();
        let json = serde_json::json!({
            "time": now.format("%H:%M:%S").to_string(),
            "date": now.format("%d.%m.%Y").to_string(),
            "hours": now.format("%H").to_string().parse::<u32>().unwrap_or(0),
            "minutes": now.format("%M").to_string().parse::<u32>().unwrap_or(0),
            "seconds": now.format("%S").to_string().parse::<u32>().unwrap_or(0),
            "timestamp": now.timestamp_millis()
        });
        json.to_string()
    })
    .unwrap();
    globals
        .set(obfstr!("host_get_local_time"), get_local_time_func)
        .unwrap();

    // Register UI manipulate bindings
    bindings_ui::register_ui_bindings(
        ctx,
        &globals,
        ui_tree,
        &plugin_permissions,
        granted_permissions.clone(),
        cache_path,
    );

    // Register App & IPC bindings
    bindings_app::register_app_bindings(
        ctx,
        &globals,
        plugin_id.clone(),
        msg_tx,
        action_queue,
        api_map,
        broadcast_queue,
        &plugin_permissions,
        granted_permissions,
    );

    // Register Native Data Vault bindings
    bindings_vault::register_vault_bindings(ctx, &globals, plugin_id, vault);

    // Register Native Hybrid Package & Component bindings (sniffer_pkg)
    bindings_pkg::register_pkg_bindings(ctx, &globals, pkg_registry);
}
