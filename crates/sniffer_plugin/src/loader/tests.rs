use crate::loader::{PluginLoader, PluginManifest};

#[test]
fn validate_manifest_accepts_basic_plugin_manifest() {
    let manifest = PluginManifest {
        id: "com.example.plugin".to_string(),
        name: "Example Plugin".to_string(),
        version: "1.0.0".to_string(),
        main: "main.js".to_string(),
        scripts: vec![],
        is_master: false,
        preload: vec![],
        permissions: vec![
            "android.permission.CAMERA".to_string(),
            "shared_view.provider".to_string(),
        ],
        default_settings: serde_json::Value::Null,
    };

    let issues = PluginLoader::validate_manifest(&manifest);
    assert!(issues.is_empty());
}

#[test]
fn test_load_all_plugins_from_disk() {
    let mut registry = crate::PluginRegistry::default();
    let queue = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let plugins_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.plugins");
    let loader = PluginLoader::new(plugins_dir.to_str().unwrap_or(".plugins"));
    loader.register_all(&mut registry, &queue);
}

#[test]
fn test_eval_framework_file_in_quickjs() {
    let rt = rquickjs::Runtime::new().unwrap();
    let ctx = rquickjs::Context::full(&rt).unwrap();

    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.plugins/framework/sniffer_ui.js");
    if let Ok(code) = std::fs::read_to_string(&path) {
        ctx.with(|c| {
            let res = c.eval::<rquickjs::Value, _>(code.as_bytes());
            if let Err(e) = res {
                let caught = c.catch();
                let exc = caught.as_exception();
                let msg = exc.as_ref().and_then(|x| x.message()).unwrap_or_default();
                let stack = exc.as_ref().and_then(|x| x.stack()).unwrap_or_default();
                panic!("File {path:?} failed: {msg}\n{stack}\n{e}");
            }
        });
    }
}

#[test]
fn validate_manifest_rejects_malicious_path_traversal() {
    let manifest = PluginManifest {
        id: "com.example.evil".to_string(),
        name: "Evil Plugin".to_string(),
        version: "1.0.0".to_string(),
        main: "../../etc/passwd.js".to_string(),
        scripts: vec!["../secret.js".to_string()],
        is_master: false,
        preload: vec!["../../outside.js".to_string()],
        permissions: vec!["plugin.permission.UI".to_string()],
        default_settings: serde_json::Value::Null,
    };

    let issues = PluginLoader::validate_manifest(&manifest);
    assert!(!issues.is_empty(), "Path traversal must be rejected");
    assert!(issues.iter().any(|i| i.contains("traversal") || i.contains("invalid")));
}

#[test]
fn validate_manifest_rejects_invalid_id_and_semver() {
    let manifest = PluginManifest {
        id: "invalid_id_no_dot".to_string(),
        name: "Bad ID Plugin".to_string(),
        version: "not_a_semver".to_string(),
        main: "main.js".to_string(),
        scripts: vec![],
        is_master: false,
        preload: vec![],
        permissions: vec!["plugin.permission.UI".to_string()],
        default_settings: serde_json::Value::Null,
    };

    let issues = PluginLoader::validate_manifest(&manifest);
    assert!(!issues.is_empty());
    assert!(issues.iter().any(|i| i.contains("id")));
    assert!(issues.iter().any(|i| i.contains("version")));
}

#[test]
fn validate_manifest_rejects_duplicate_and_bad_permissions() {
    let manifest = PluginManifest {
        id: "com.example.permtest".to_string(),
        name: "Perm Test".to_string(),
        version: "1.0.0".to_string(),
        main: "main.js".to_string(),
        scripts: vec![],
        is_master: false,
        preload: vec![],
        permissions: vec![
            "plugin.permission.UI".to_string(),
            "plugin.permission.UI".to_string(),
            "badperm".to_string(),
        ],
        default_settings: serde_json::Value::Null,
    };

    let issues = PluginLoader::validate_manifest(&manifest);
    assert!(!issues.is_empty());
    assert!(issues.iter().any(|i| i.contains("duplicate")));
    assert!(issues.iter().any(|i| i.contains("namespace")));
}

#[test]
fn validate_manifest_rejects_non_object_default_settings() {
    let manifest = PluginManifest {
        id: "com.example.settingstest".to_string(),
        name: "Settings Test".to_string(),
        version: "1.0.0".to_string(),
        main: "main.js".to_string(),
        scripts: vec![],
        is_master: false,
        preload: vec![],
        permissions: vec!["plugin.permission.UI".to_string()],
        default_settings: serde_json::json!([1, 2, 3]),
    };

    let issues = PluginLoader::validate_manifest(&manifest);
    assert!(!issues.is_empty());
    assert!(issues.iter().any(|i| i.contains("defaultSettings")));
}
