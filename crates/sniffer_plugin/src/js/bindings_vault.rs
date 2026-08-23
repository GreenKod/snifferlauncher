//! QuickJS Host Bindings for the Native Data Vault Engine

use obfstr::obfstr;
use rquickjs::{Ctx, Function, Object};
use sniffer_core::vault::{DataVault, QueryAppsParams};
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
    let plugin_id_get = plugin_id.clone();
    let get_func = Function::new(ctx.clone(), move |key: String| -> Option<String> {
        vault_get.get(&key, &plugin_id_get)
    })
    .unwrap();
    globals.set(obfstr!("host_vault_get"), get_func).unwrap();

    // host_vault_set
    let vault_set = Arc::clone(&vault);
    let plugin_id_set = plugin_id.clone();
    let set_func = Function::new(ctx.clone(), move |key: String, value: String| -> bool {
        vault_set.set(&key, &value, &plugin_id_set).is_ok()
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
        let result = vault_query.query_apps(&params);
        serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string())
    })
    .unwrap();
    globals
        .set(obfstr!("host_vault_query_apps"), query_apps_func)
        .unwrap();

    // host_vault_keys
    let vault_keys = Arc::clone(&vault);
    let plugin_id_keys = plugin_id.clone();
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

    // host_vault_save_file
    let vault_save_file = Arc::clone(&vault);
    let plugin_id_save = plugin_id.clone();
    let save_file_func = Function::new(
        ctx.clone(),
        move |file_name: String, base64_content: String| -> String {
            // Strip data URL prefix if present (e.g. data:image/png;base64,)
            let clean_b64 = if let Some(idx) = base64_content.find(";base64,") {
                &base64_content[idx + 8..]
            } else {
                &base64_content
            };

            let bytes = match decode_base64(clean_b64) {
                Some(b) => b,
                None => return String::new(),
            };

            vault_save_file
                .save_file(&file_name, &bytes, &plugin_id_save)
                .unwrap_or_default()
        },
    )
    .unwrap();
    globals
        .set(obfstr!("host_vault_save_file"), save_file_func)
        .unwrap();

    // host_vault_read_file
    let vault_read_file = Arc::clone(&vault);
    let plugin_id_read = plugin_id.clone();
    let read_file_func = Function::new(ctx.clone(), move |file_name: String| -> Option<String> {
        let bytes = vault_read_file.read_file(&file_name, &plugin_id_read)?;
        Some(encode_base64(&bytes))
    })
    .unwrap();
    globals
        .set(obfstr!("host_vault_read_file"), read_file_func)
        .unwrap();

    // host_vault_delete_file
    let vault_delete_file = Arc::clone(&vault);
    let plugin_id_del_file = plugin_id.clone();
    let delete_file_func = Function::new(ctx.clone(), move |file_name: String| -> bool {
        vault_delete_file
            .delete_file(&file_name, &plugin_id_del_file)
            .unwrap_or(false)
    })
    .unwrap();
    globals
        .set(obfstr!("host_vault_delete_file"), delete_file_func)
        .unwrap();

    // host_vault_list_files
    let vault_list_files = Arc::clone(&vault);
    let plugin_id_list = plugin_id;
    let list_files_func = Function::new(ctx.clone(), move || -> String {
        let files = vault_list_files.list_files(&plugin_id_list);
        serde_json::to_string(&files).unwrap_or_else(|_| "[]".to_string())
    })
    .unwrap();
    globals
        .set(obfstr!("host_vault_list_files"), list_files_func)
        .unwrap();
}

/// Simple RFC4648 Base64 decoder without external dependencies.
fn decode_base64(input: &str) -> Option<Vec<u8>> {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut buffer = 0u32;
    let mut bits = 0;
    let mut output = Vec::new();

    for &b in input.as_bytes() {
        if b == b'=' || b.is_ascii_whitespace() {
            continue;
        }
        let pos = TABLE.iter().position(|&c| c == b)?;
        let val = pos as u32;
        buffer = (buffer << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    Some(output)
}

/// Simple RFC4648 Base64 encoder without external dependencies.
fn encode_base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::new();
    let mut buffer = 0u32;
    let mut bits = 0;

    for &b in bytes {
        buffer = (buffer << 8) | (b as u32);
        bits += 8;
        while bits >= 6 {
            bits -= 6;
            let idx = ((buffer >> bits) & 0x3F) as usize;
            output.push(TABLE[idx] as char);
        }
    }
    if bits > 0 {
        buffer <<= 6 - bits;
        let idx = (buffer & 0x3F) as usize;
        output.push(TABLE[idx] as char);
        while output.len() % 4 != 0 {
            output.push('=');
        }
    }
    output
}
