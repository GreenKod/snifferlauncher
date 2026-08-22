//! Integration tests for QuickJS Host Bridge & Native Package Layer (`sniffer_pkg`).

use rquickjs::{Context, Runtime};
use sniffer_core::vault::DataVault;
use sniffer_pkg::PackageRegistry;
use sniffer_pkg::perf_monitor::PerfMonitorPackage;
use sniffer_pkg::scroll_view::ScrollViewPackage;
use sniffer_plugin::js::bindings_pkg::register_pkg_bindings;
use std::sync::{Arc, RwLock};

#[test]
fn test_host_pkg_query_perf_monitor() {
    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();

    let vault = Arc::new(DataVault::default());
    let mut pkg_reg = PackageRegistry::new(vault);

    // Register PerfMonitorPackage
    pkg_reg.register_service(Arc::new(PerfMonitorPackage::new()));

    let pkg_registry = Arc::new(RwLock::new(pkg_reg));

    ctx.with(|c| {
        let globals = c.globals();
        register_pkg_bindings(&c, &globals, Some(pkg_registry));

        // 1. Query getCpuUsage
        let cpu_json_str: String = c
            .eval(r#"host_pkg_query("com.sniffer.perf", "getCpuUsage", "{}")"#)
            .unwrap();
        let cpu_val: serde_json::Value =
            serde_json::from_str(&cpu_json_str).expect("getCpuUsage must return valid JSON");
        assert_eq!(cpu_val["ok"], true);
        assert!(cpu_val["cpu_usage"].is_number());

        // 2. Query getAllMetrics
        let all_json_str: String = c
            .eval(r#"host_pkg_query("com.sniffer.perf", "getAllMetrics", "{}")"#)
            .unwrap();
        let all_val: serde_json::Value =
            serde_json::from_str(&all_json_str).expect("getAllMetrics must return valid JSON");
        assert_eq!(all_val["ok"], true);
        assert!(all_val["cpu_usage"].is_number());
        assert!(all_val["mem_mb"].is_number());
        assert!(all_val["fps"].is_number());

        // 3. Query setPollingInterval
        let set_json_str: String = c
            .eval(
                r#"host_pkg_query("com.sniffer.perf", "setPollingInterval", JSON.stringify({ interval_ms: 750 }))"#,
            )
            .unwrap();
        let set_val: serde_json::Value =
            serde_json::from_str(&set_json_str).expect("setPollingInterval must return valid JSON");
        assert_eq!(set_val["ok"], true);
        assert_eq!(set_val["interval_ms"], 750);

        // 4. Query unknown package -> returns JSON error object
        let err_json_str: String = c
            .eval(r#"host_pkg_query("com.unknown.service", "getCpuUsage", "{}")"#)
            .unwrap();
        let err_val: serde_json::Value =
            serde_json::from_str(&err_json_str).expect("unknown pkg query must return valid JSON");
        assert!(err_val["error"].is_string());
    });
}

#[test]
fn test_host_extend_widget_scroll_view() {
    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();

    let vault = Arc::new(DataVault::default());
    let mut pkg_reg = PackageRegistry::new(vault);

    // Register ScrollViewPackage
    let scroll_view = Arc::new(ScrollViewPackage::new());
    pkg_reg.register_widget(scroll_view);

    let pkg_registry = Arc::new(RwLock::new(pkg_reg));

    ctx.with(|c| {
        let globals = c.globals();
        register_pkg_bindings(&c, &globals, Some(pkg_registry));

        // 1. Extend widget with valid descriptor
        let ok_json_str: String = c
            .eval(
                r#"
                host_extend_widget("com.sniffer.scroll_view", JSON.stringify({
                    snap_x: 360.0,
                    page_count: 4,
                    rubber_band: 0.45
                }))
            "#,
            )
            .unwrap();
        let ok_val: serde_json::Value =
            serde_json::from_str(&ok_json_str).expect("host_extend_widget must return valid JSON");
        assert_eq!(ok_val["ok"], true);

        // 2. Extend widget with invalid JSON descriptor
        let bad_json_str: String = c
            .eval(r#"host_extend_widget("com.sniffer.scroll_view", "not valid json")"#)
            .unwrap();
        let bad_val: serde_json::Value =
            serde_json::from_str(&bad_json_str).expect("invalid descriptor must return valid JSON");
        assert_eq!(bad_val["ok"], false);
        assert!(bad_val["error"].is_string());

        // 3. Extend non-existent widget
        let missing_json_str: String = c
            .eval(r#"host_extend_widget("com.nonexistent.widget", "{}")"#)
            .unwrap();
        let missing_val: serde_json::Value =
            serde_json::from_str(&missing_json_str).expect("missing widget must return valid JSON");
        assert_eq!(missing_val["ok"], false);
        assert!(missing_val["error"].is_string());
    });
}

#[test]
fn test_host_pkg_bridge_fallback_when_registry_none() {
    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();

    ctx.with(|c| {
        let globals = c.globals();
        register_pkg_bindings(&c, &globals, None);

        // Query when registry is None
        let query_res: String = c
            .eval(r#"host_pkg_query("com.sniffer.perf", "getCpuUsage", "{}")"#)
            .unwrap();
        let query_val: serde_json::Value = serde_json::from_str(&query_res).unwrap();
        assert!(query_val["error"].is_string());

        // Extend widget when registry is None
        let extend_res: String = c
            .eval(r#"host_extend_widget("com.sniffer.scroll_view", "{}")"#)
            .unwrap();
        let extend_val: serde_json::Value = serde_json::from_str(&extend_res).unwrap();
        assert_eq!(extend_val["ok"], false);
        assert!(extend_val["error"].is_string());
    });
}
