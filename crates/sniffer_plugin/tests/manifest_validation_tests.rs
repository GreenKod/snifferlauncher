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
                let content = fs::read_to_string(&script_file)
                    .unwrap_or_else(|e| panic!("Failed to read {script_file:?}: {e}"));
                let normalized = content.replace("\r\n", "\n");
                let actual_hash =
                    sniffer_pkg::manifest::sha256::compute_sha256_hex(normalized.as_bytes());
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
                let content = fs::read_to_string(&preload_path)
                    .unwrap_or_else(|e| panic!("Failed to read {preload_path:?}: {e}"));
                let normalized = content.replace("\r\n", "\n");
                let actual_hash =
                    sniffer_pkg::manifest::sha256::compute_sha256_hex(normalized.as_bytes());
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

#[test]
fn test_plugin_loader_normalizes_crlf_checksum() {
    let temp_dir = std::env::temp_dir().join(format!("sniffer_test_crlf_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(temp_dir.join("crlf_plugin")).expect("Failed to create temp dir");

    let plugins_json = r#"{ "active_plugins": ["crlf_plugin"] }"#;
    fs::write(temp_dir.join("plugins.json"), plugins_json).expect("Failed to write plugins.json");

    // The code with LF has a known hash
    let lf_code = "const x = 1;\nconst y = 2;\n";
    let lf_hash = sniffer_pkg::manifest::sha256::compute_sha256_hex(lf_code.as_bytes());

    let manifest_json = format!(
        r#"{{
        "id": "com.sniffer.crlf",
        "name": "CRLF Plugin",
        "version": "1.0.0",
        "main": "main.js",
        "permissions": ["plugin.permission.UI"],
        "checksums": {{
            "main.js": "{lf_hash}"
        }}
    }}"#
    );
    fs::write(temp_dir.join("crlf_plugin/manifest.json"), manifest_json)
        .expect("Failed to write manifest.json");

    // Write file with Windows CRLF (\r\n) line endings on disk
    let crlf_code = "const x = 1;\r\nconst y = 2;\r\n";
    fs::write(temp_dir.join("crlf_plugin/main.js"), crlf_code).expect("Failed to write main.js");

    let mut registry = sniffer_plugin::PluginRegistry::default();
    let action_queue = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let loader = sniffer_plugin::loader::PluginLoader::new(&temp_dir);
    loader.register_all(&mut registry, &action_queue);

    // Plugin MUST be successfully registered because CRLF was normalized to LF!
    assert_eq!(
        registry.len(),
        1,
        "Plugin with CRLF line endings should be accepted after normalization"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_dock_plugin_spring_pressed_and_elevation_damping() {
    use rquickjs::{Context, Runtime};

    let plugins_dir = get_plugins_dir();
    let dock_dir = plugins_dir.join("dock");
    let framework_path = plugins_dir.join("framework/sniffer_ui.js");

    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();

    ctx.with(|c| {
        // Setup mock environment
        let setup = r"
            globalThis.host_log = function() {};
            globalThis.host_screen_width = function() { return 1080; };
            globalThis.host_screen_height = function() { return 2400; };
            globalThis.requestPermissions = function(p) { return p; };
            globalThis.registeredApis = {};
            globalThis.registerApi = function(name, fn) { globalThis.registeredApis[name] = fn; };
            globalThis.callApi = function(name, payload) { return globalThis.registeredApis[name](payload); };
            globalThis.broadcastEvent = function() {};
            globalThis.subscribeChannel = function() {};
            globalThis.host_hash = function(s) {
                let h = 0x811c9dc5;
                for (let i = 0; i < s.length; i++) {
                    h ^= s.charCodeAt(i);
                    h = Math.imul(h, 0x01000193);
                }
                return (h >>> 0);
            };
        ";
        c.eval::<(), _>(setup).unwrap();

        // Evaluate framework
        let fw_code = fs::read_to_string(&framework_path).unwrap();
        c.eval::<(), _>(fw_code).unwrap();

        // Evaluate dock files
        let apps_code = fs::read_to_string(dock_dir.join("services/apps.js")).unwrap();
        c.eval::<(), _>(apps_code).unwrap();

        let dock_bar_code = fs::read_to_string(dock_dir.join("components/dock_bar.js")).unwrap();
        c.eval::<(), _>(dock_bar_code).unwrap();

        let main_code = fs::read_to_string(dock_dir.join("main.js")).unwrap();
        c.eval::<(), _>(main_code).unwrap();

        // 1. Get resting dock UI
        let ui_json_str: String = c
            .eval("JSON.stringify(globalThis.registeredApis['dock.getUI']({ isLandscape: false }))")
            .unwrap();
        let ui: sniffer_core::types::Element = serde_json::from_str(&ui_json_str).unwrap();


        fn find_circle(
            el: &sniffer_core::types::Element,
            target_id: &str,
        ) -> Option<sniffer_core::style::Style> {
            match el {
                sniffer_core::types::Element::Container {
                    id,
                    style,
                    children,
                } => {
                    if id.as_deref() == Some(target_id) {
                        return Some(style.clone());
                    }
                    for child in children {
                        if let Some(s) = find_circle(child, target_id) {
                            return Some(s);
                        }
                    }
                    None
                }
                _ => None,
            }
        }

        let phone_style =
            find_circle(&ui, "dock_app_circle_phone").expect("dock_app_circle_phone found");
        assert_eq!(phone_style.elevation, 6.0);
        assert_eq!(phone_style.transform.scale, 1.0);
        assert_eq!(phone_style.shadow_blur, 10.0);
        match &phone_style.transition.easing {
            sniffer_core::style::Easing::Spring { stiffness, damping } => {
                assert_eq!(*stiffness, 320.0);
                assert_eq!(*damping, 22.0);
            }
            other => panic!("expected Spring easing, got {other:?}"),
        }

        // 2. Trigger PointerDown on phone button
        c.eval::<(), _>(
            "globalThis.onEvent(JSON.stringify({ type: 'PointerDown', id: String(host_hash('dock_app_btn_phone')) }));",
        )
        .unwrap();

        // UI in pressed state
        let ui_pressed_str: String = c
            .eval("JSON.stringify(globalThis.registeredApis['dock.getUI']({ isLandscape: false }))")
            .unwrap();
        let ui_pressed: sniffer_core::types::Element =
            serde_json::from_str(&ui_pressed_str).unwrap();
        let phone_pressed_style = find_circle(&ui_pressed, "dock_app_circle_phone")
            .expect("dock_app_circle_phone found in pressed state");
        assert_eq!(phone_pressed_style.elevation, 2.0);
        assert_eq!(phone_pressed_style.transform.scale, 0.88);
        assert_eq!(phone_pressed_style.shadow_blur, 4.0);

        // Messages icon should remain unpressed
        let messages_style =
            find_circle(&ui_pressed, "dock_app_circle_messages").expect("messages found");
        assert_eq!(messages_style.elevation, 6.0);
        assert_eq!(messages_style.transform.scale, 1.0);

        // 3. Trigger PointerUp
        c.eval::<(), _>(
            "globalThis.onEvent(JSON.stringify({ type: 'PointerUp', id: null }));",
        )
        .unwrap();
        let ui_released_str: String = c
            .eval("JSON.stringify(globalThis.registeredApis['dock.getUI']({ isLandscape: false }))")
            .unwrap();
        let ui_released: sniffer_core::types::Element =
            serde_json::from_str(&ui_released_str).unwrap();
        let phone_released_style = find_circle(&ui_released, "dock_app_circle_phone")
            .expect("dock_app_circle_phone found in released state");
        assert_eq!(phone_released_style.elevation, 6.0);
        assert_eq!(phone_released_style.transform.scale, 1.0);
    });
}
