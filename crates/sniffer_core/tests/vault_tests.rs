use sniffer_core::types::AppInfo;
use sniffer_core::vault::{DataVault, QueryAppsParams};

#[test]
fn test_data_vault_basic_crud() {
    let vault = DataVault::new(None);

    assert!(vault.get("plugin.plugin_a.test").is_none());

    let res = vault.set("plugin.plugin_a.test", "my_value".to_string(), "plugin_a");
    assert!(res.is_ok());
    assert_eq!(
        vault.get("plugin.plugin_a.test").as_deref(),
        Some("my_value")
    );

    let rem_res = vault.delete("plugin.plugin_a.test", "plugin_a");
    assert!(rem_res.is_ok());
    assert!(vault.get("plugin.plugin_a.test").is_none());
}

#[test]
fn test_data_vault_apps_query_and_paging() {
    let vault = DataVault::new(None);

    let apps = vec![
        AppInfo::new("Alpha".to_string(), "com.app.a".to_string()),
        AppInfo::new("Beta".to_string(), "com.app.b".to_string()),
        AppInfo::new("Charlie".to_string(), "com.app.c".to_string()),
        AppInfo::new("Delta".to_string(), "com.app.d".to_string()),
    ];

    vault.update_system_apps(apps);

    // Paging limit 2, page 0
    let res_page0 = vault.query_apps(QueryAppsParams {
        page: Some(0),
        limit: Some(2),
        search: None,
    });

    assert_eq!(res_page0.total_count, 4);
    assert_eq!(res_page0.total_pages, 2);
    assert_eq!(res_page0.page, 0);
    assert_eq!(res_page0.apps.len(), 2);
    assert_eq!(res_page0.apps[0].name, "Alpha");
    assert_eq!(res_page0.apps[1].name, "Beta");

    // Search query
    let res_search = vault.query_apps(QueryAppsParams {
        page: Some(0),
        limit: Some(10),
        search: Some("del".to_string()),
    });

    assert_eq!(res_search.total_count, 1);
    assert_eq!(res_search.apps[0].name, "Delta");
}
