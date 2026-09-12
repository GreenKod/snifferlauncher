#![cfg(not(target_os = "android"))]

use std::fs;
use std::path::{Path, PathBuf};

fn get_plugins_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.plugins")
        .canonicalize()
        .expect("Could not find .plugins directory")
}

#[test]
fn test_all_plugin_manifests_are_valid_and_files_exist() {
    let plugins_dir = get_plugins_dir();
    let plugins_json_path = plugins_dir.join("plugins.json");
    assert!(
        plugins_json_path.exists(),
        "plugins.json does not exist at {plugins_json_path:?}"
    );

    let config_content =
        fs::read_to_string(&plugins_json_path).expect("Failed to read plugins.json");
    let config: sniffer_plugin::loader::PluginsConfig =
        serde_json::from_str(&config_content).expect("Failed to parse plugins.json");

    let mut plugin_folders = config.active_plugins;
    if let Some(master) = config.master_plugin {
        if !plugin_folders.contains(&master) {
            plugin_folders.push(master);
        }
    }

    assert!(
        !plugin_folders.is_empty(),
        "active_plugins in plugins.json should not be empty"
    );

    for folder in plugin_folders {
        let plugin_path = plugins_dir.join(&folder);
        assert!(
            plugin_path.is_dir(),
            "Plugin folder does not exist: {plugin_path:?}"
        );

        let manifest_path = plugin_path.join("manifest.json");
        assert!(
            manifest_path.exists(),
            "manifest.json missing for plugin {folder}"
        );

        let manifest_str = fs::read_to_string(&manifest_path)
            .unwrap_or_else(|e| panic!("Failed to read {manifest_path:?}: {e}"));
        let manifest: sniffer_plugin::loader::PluginManifest = serde_json::from_str(&manifest_str)
            .unwrap_or_else(|e| panic!("Invalid JSON schema in {manifest_path:?}: {e}"));

        let issues = manifest.validate();
        assert!(
            issues.is_empty(),
            "Validation errors in {manifest_path:?}: {issues:?}"
        );

        // Verify main entry file exists
        let main_file = plugin_path.join(&manifest.main);
        let main_entry = &manifest.main;
        assert!(
            main_file.exists(),
            "Main entry file {main_entry:?} not found for plugin {folder}"
        );

        // Verify all scripts exist and checksums match if declared
        for script in &manifest.scripts {
            let script_file = plugin_path.join(script);
            assert!(
                script_file.exists(),
                "Script {script:?} not found for plugin {folder}"
            );

            if let Some(expected_hash) = manifest.checksums.get(script) {
                let bytes = fs::read(&script_file)
                    .unwrap_or_else(|e| panic!("Failed to read {script_file:?}: {e}"));
                let actual_hash = sniffer_pkg::manifest::sha256::compute_sha256_hex(&bytes);
                assert_eq!(
                    &actual_hash, expected_hash,
                    "Checksum mismatch for script {script:?} in {folder}"
                );
            }
        }

        // Verify preloads and checksums
        for preload_rel in &manifest.preload {
            let preload_path = if let Some(stripped) = preload_rel.strip_prefix("../") {
                plugins_dir.join(stripped)
            } else {
                plugin_path.join(preload_rel)
            };
            assert!(
                preload_path.exists(),
                "Preload {preload_path:?} not found for plugin {folder}"
            );
            if let Some(expected_hash) = manifest.checksums.get(preload_rel) {
                let bytes = fs::read(&preload_path)
                    .unwrap_or_else(|e| panic!("Failed to read {preload_path:?}: {e}"));
                let actual_hash = sniffer_pkg::manifest::sha256::compute_sha256_hex(&bytes);
                assert_eq!(
                    &actual_hash, expected_hash,
                    "Checksum mismatch for preload {preload_rel:?} in {folder}"
                );
            }
        }
    }
}

#[test]
fn test_all_plugin_and_framework_javascript_syntax() {
    let plugins_dir = get_plugins_dir();
    let rt = rquickjs::Runtime::new().expect("Failed to create QuickJS runtime");
    let ctx = rquickjs::Context::full(&rt).expect("Failed to create QuickJS context");

    let mut js_files = Vec::new();
    fn visit_dirs(dir: &Path, js_files: &mut Vec<PathBuf>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    visit_dirs(&path, js_files);
                } else if path.extension().and_then(|s| s.to_str()) == Some("js") {
                    js_files.push(path);
                }
            }
        }
    }

    visit_dirs(&plugins_dir, &mut js_files);
    assert!(
        !js_files.is_empty(),
        "No JS files found in .plugins directory"
    );

    for file in js_files {
        let code =
            fs::read_to_string(&file).unwrap_or_else(|e| panic!("Failed to read {file:?}: {e}"));

        ctx.with(|c| {
            // Evaluates code in QuickJS to verify valid ECMAScript syntax
            let res = c.eval::<rquickjs::Value, _>(code.as_bytes());
            if let Err(e) = res {
                // Ignore runtime reference errors caused by un-injected host globals,
                // but fail on actual syntax/parse errors.
                let caught = c.catch();
                let exc = caught.as_exception();
                let msg = exc.as_ref().and_then(|x| x.message()).unwrap_or_default();
                assert!(
                    !msg.contains("SyntaxError"),
                    "Syntax error in JavaScript file {file:?}: {msg}\n{e}"
                );
            }
        });
    }
}

#[test]
fn test_plugin_loader_rejects_tampered_checksum() {
    let temp_dir = std::env::temp_dir().join(format!("sniffer_test_tamper_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(temp_dir.join("tampered_plugin")).expect("Failed to create temp dir");

    let plugins_json = r#"{ "active_plugins": ["tampered_plugin"] }"#;
    fs::write(temp_dir.join("plugins.json"), plugins_json).expect("Failed to write plugins.json");

    let manifest_json = r#"{
        "id": "com.sniffer.tampered",
        "name": "Tampered Plugin",
        "version": "1.0.0",
        "main": "main.js",
        "permissions": ["plugin.permission.UI"],
        "checksums": {
            "main.js": "0000000000000000000000000000000000000000000000000000000000000000"
        }
    }"#;
    fs::write(
        temp_dir.join("tampered_plugin/manifest.json"),
        manifest_json,
    )
    .expect("Failed to write manifest.json");

    let js_code = "const x = 123;";
    fs::write(temp_dir.join("tampered_plugin/main.js"), js_code).expect("Failed to write main.js");

    let mut registry = sniffer_plugin::PluginRegistry::default();
    let action_queue = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let loader = sniffer_plugin::loader::PluginLoader::new(&temp_dir);
    loader.register_all(&mut registry, &action_queue);

    // Plugin MUST NOT be registered due to checksum mismatch
    assert_eq!(
        registry.len(),
        0,
        "Tampered plugin should have been rejected by the loader"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}
