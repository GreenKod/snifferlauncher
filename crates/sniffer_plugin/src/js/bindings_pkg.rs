//! QuickJS Host Bindings for the Native Hybrid Package & Component Layer (`sniffer_pkg`).
//!
//! Exposes host functions allowing JS plugins to query background service packages
//! and dynamically configure native GPU widgets:
//!
//! - `host_pkg_query(pkg_id, method, payload_json) -> string`
//! - `host_extend_widget(widget_id, descriptor_json) -> string`

use obfstr::obfstr;
use rquickjs::{Ctx, Function, Object};
use sniffer_pkg::PackageRegistry;
use std::sync::{Arc, RwLock};

/// Register package bridge host functions into the QuickJS environment.
pub fn register_pkg_bindings<'js>(
    ctx: &Ctx<'js>,
    globals: &Object<'js>,
    pkg_registry: Option<Arc<RwLock<PackageRegistry>>>,
) {
    // -----------------------------------------------------------------------
    // host_pkg_query(pkg_id: string, method: string, payload_json: string) -> string
    // -----------------------------------------------------------------------
    let reg_query = pkg_registry.clone();
    let query_func = Function::new(
        ctx.clone(),
        move |pkg_id: String, method: String, payload_json: String| -> String {
            if let Some(ref reg) = reg_query {
                match reg.read() {
                    Ok(guard) => match guard.query_service(&pkg_id, &method, &payload_json) {
                        Ok(resp) => resp,
                        Err(e) => serde_json::json!({
                            "error": e.to_string()
                        })
                        .to_string(),
                    },
                    Err(_) => serde_json::json!({
                        "error": "Package registry lock poisoned"
                    })
                    .to_string(),
                }
            } else {
                serde_json::json!({
                    "error": "Package registry not available"
                })
                .to_string()
            }
        },
    )
    .unwrap();
    globals.set(obfstr!("host_pkg_query"), query_func).unwrap();

    // -----------------------------------------------------------------------
    // host_extend_widget(widget_id: string, descriptor_json: string) -> string
    // -----------------------------------------------------------------------
    let reg_extend = pkg_registry;
    let extend_func = Function::new(
        ctx.clone(),
        move |widget_id: String, descriptor_json: String| -> String {
            if let Some(ref reg) = reg_extend {
                match reg.read() {
                    Ok(guard) => {
                        match guard.apply_widget_descriptor(&widget_id, &descriptor_json) {
                            Ok(()) => serde_json::json!({
                                "ok": true
                            })
                            .to_string(),
                            Err(e) => serde_json::json!({
                                "ok": false,
                                "error": e.to_string()
                            })
                            .to_string(),
                        }
                    }
                    Err(_) => serde_json::json!({
                        "ok": false,
                        "error": "Package registry lock poisoned"
                    })
                    .to_string(),
                }
            } else {
                serde_json::json!({
                    "ok": false,
                    "error": "Package registry not available"
                })
                .to_string()
            }
        },
    )
    .unwrap();
    globals
        .set(obfstr!("host_extend_widget"), extend_func)
        .unwrap();
}
