//! QuickJS Host Bindings for the Native Data Vault Engine

use crate::core::vault::{DataVault, QueryAppsParams};
use obfstr::obfstr;
use rquickjs::{Ctx, Function, Object};
use std::sync::Arc;

/// Register all DataVault host functions into the QuickJS environment.
pub fn register_vault_bindings<'js>(
    ctx: &Ctx<'js>,
    globals: &Object<'js>,
    plugin_id: String,
    vault: Arc<DataVault>,
) {
    // host_vault_get
    let vault_get = Arc::clone(&vault);
    let get_func = Function::new(ctx.clone(), move |key: String| -> Option<String> {
        vault_get.get(&key)
    })
    .unwrap();
    globals.set(obfstr!("host_vault_get"), get_func).unwrap();

    // host_vault_set
    let vault_set = Arc::clone(&vault);
    let plugin_id_set = plugin_id.clone();
    let set_func = Function::new(ctx.clone(), move |key: String, value: String| -> bool {
        vault_set.set(&key, value, &plugin_id_set).is_ok()
    })
    .unwrap();
    globals.set(obfstr!("host_vault_set"), set_func).unwrap();

    // host_vault_delete
    let vault_del = Arc::clone(&vault);
    let plugin_id_del = plugin_id.clone();
    let del_func = Function::new(ctx.clone(), move |key: String| -> bool {
        vault_del.delete(&key, &plugin_id_del).unwrap_or(false)
    })
    .unwrap();
    globals.set(obfstr!("host_vault_delete"), del_func).unwrap();

    // host_vault_query_apps
    let vault_query = Arc::clone(&vault);
    let query_apps_func = Function::new(ctx.clone(), move |params_json: String| -> String {
        let params: QueryAppsParams = serde_json::from_str(&params_json).unwrap_or_default();
        let result = vault_query.query_apps(params);
        serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string())
    })
    .unwrap();
    globals
        .set(obfstr!("host_vault_query_apps"), query_apps_func)
        .unwrap();

    // host_vault_keys
    let vault_keys = Arc::clone(&vault);
    let plugin_id_keys = plugin_id;
    let keys_func = Function::new(ctx.clone(), move |prefix: String| -> String {
        let prefix_opt = if prefix.is_empty() {
            None
        } else {
            Some(prefix.as_str())
        };
        let keys = vault_keys.keys(prefix_opt, &plugin_id_keys);
        serde_json::to_string(&keys).unwrap_or_else(|_| "[]".to_string())
    })
    .unwrap();
    globals.set(obfstr!("host_vault_keys"), keys_func).unwrap();
}
