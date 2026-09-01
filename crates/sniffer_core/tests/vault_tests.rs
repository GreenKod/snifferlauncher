use serde::{Deserialize, Serialize};
use sniffer_core::types::{AppInfo, QueryAppsParams, query_apps};
use sniffer_core::vault::DataVault;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct TestConfig {
    enabled: bool,
    threshold: u32,
}

#[test]
fn test_data_vault_basic_crud_and_json() {
    let vault = DataVault::new(None);

    assert!(vault.get("plugin.plugin_a.test", "plugin_a").is_none());

    let res = vault.set("plugin.plugin_a.test", "my_value", "plugin_a");
    assert!(res.is_ok());
    assert_eq!(
        vault.get("plugin.plugin_a.test", "plugin_a").as_deref(),
        Some("my_value")
    );

    // Generic JSON set/get
    let cfg = TestConfig {
        enabled: true,
        threshold: 42,
    };
    assert!(vault.set_json("settings", &cfg, "plugin_a").is_ok());
    let retrieved: Option<TestConfig> = vault.get_json("settings", "plugin_a");
    assert_eq!(retrieved, Some(cfg));

    let rem_res = vault.delete("plugin.plugin_a.test", "plugin_a");
    assert!(rem_res.is_ok());
    assert!(vault.get("plugin.plugin_a.test", "plugin_a").is_none());
}

#[test]
fn test_data_vault_namespace_security() {
    let vault = DataVault::new(None);

    // Only "system" can write to system.* namespace
    assert!(vault.set("system.version", "1.0", "plugin_a").is_err());
    assert!(vault.set("system.version", "1.0", "system").is_ok());

    // Plugin A cannot mutate Plugin B keys
    assert!(vault.set("plugin.plugin_b.key", "val", "plugin_a").is_err());
}

#[test]
fn test_standalone_apps_query_and_paging() {
    let apps = vec![
        AppInfo::new("Alpha".to_string(), "com.app.a".to_string()),
        AppInfo::new("Beta".to_string(), "com.app.b".to_string()),
        AppInfo::new("Charlie".to_string(), "com.app.c".to_string()),
        AppInfo::new("Delta".to_string(), "com.app.d".to_string()),
    ];

    // Paging limit 2, page 0
    let res_page0 = query_apps(
        &apps,
        &QueryAppsParams {
            page: Some(0),
            limit: Some(2),
            search: None,
        },
    );

    assert_eq!(res_page0.total_count, 4);
    assert_eq!(res_page0.total_pages, 2);
    assert_eq!(res_page0.page, 0);
    assert_eq!(res_page0.apps.len(), 2);
    assert_eq!(res_page0.apps[0].name, "Alpha");
    assert_eq!(res_page0.apps[1].name, "Beta");

    // Search query
    let res_search = query_apps(
        &apps,
        &QueryAppsParams {
            page: Some(0),
            limit: Some(10),
            search: Some("del".to_string()),
        },
    );

    assert_eq!(res_search.total_count, 1);
    assert_eq!(res_search.apps[0].name, "Delta");
}
