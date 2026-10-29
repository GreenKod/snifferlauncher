#![cfg(not(target_os = "android"))]

use snifferlauncher::core as sniffer_core;
use snifferlauncher::pkg as sniffer_pkg;
use snifferlauncher::plugin::{self as sniffer_plugin, rquickjs, serde_json};
use std::fs;
use std::path::{Path, PathBuf};

fn get_plugins_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(".plugins")
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

        let main_file = plugin_path.join(&manifest.main);
        let main_entry = &manifest.main;
        assert!(
            main_file.exists(),
            "Main entry file {main_entry:?} not found for plugin {folder}"
        );

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
            let res = c.eval::<rquickjs::Value, _>(code.as_bytes());
            if let Err(e) = res {
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
fn test_dock_plugin_spring_pressed_and_elevation_damping() {
    use rquickjs::{Context, Runtime};

    let plugins_dir = get_plugins_dir();
    let dock_dir = plugins_dir.join("dock");
    let framework_path = plugins_dir.join("framework/sniffer_ui.js");

    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();

    ctx.with(|c| {
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

        let fw_code = fs::read_to_string(&framework_path).unwrap();
        c.eval::<(), _>(fw_code).unwrap();

        let apps_code = fs::read_to_string(dock_dir.join("services/apps.js")).unwrap();
        c.eval::<(), _>(apps_code).unwrap();

        let dock_bar_code = fs::read_to_string(dock_dir.join("components/dock_bar.js")).unwrap();
        c.eval::<(), _>(dock_bar_code).unwrap();

        let main_code = fs::read_to_string(dock_dir.join("main.js")).unwrap();
        c.eval::<(), _>(main_code).unwrap();

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

        c.eval::<(), _>(
            "globalThis.onEvent(JSON.stringify({ type: 'PointerDown', id: String(host_hash('dock_app_btn_phone')) }));",
        )
        .unwrap();

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

        let messages_style =
            find_circle(&ui_pressed, "dock_app_circle_messages").expect("messages found");
        assert_eq!(messages_style.elevation, 6.0);
        assert_eq!(messages_style.transform.scale, 1.0);

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

#[test]
fn test_dock_plugin_landscape_portrait_orientation_transition() {
    use rquickjs::{Context, Runtime};

    let plugins_dir = get_plugins_dir();
    let dock_dir = plugins_dir.join("dock");

    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();

    ctx.with(|c| {
        c.eval::<(), _>(
            r"
            globalThis.host_log = function() {};
            globalThis.host_screen_width = function() { return 1080; };
            globalThis.host_screen_height = function() { return 2400; };
            globalThis.requestPermissions = function(p) { return p; };
            globalThis.registeredApis = {};
            globalThis.registerApi = function(name, fn) { globalThis.registeredApis[name] = fn; };
            globalThis.callApi = function(name, payload) { return globalThis.registeredApis[name](payload); };
            globalThis.broadcastEvent = function() {};
            globalThis.subscribeChannel = function() {};
            globalThis.host_hash = function(str) {
                let hash = 0;
                for (let i = 0; i < str.length; i++) {
                    hash = ((hash << 5) - hash) + str.charCodeAt(i);
                    hash |= 0;
                }
                return Math.abs(hash);
            };
            globalThis.launchApp = function(pkg) {};
        ",
        )
        .unwrap();

        let framework_code =
            fs::read_to_string(plugins_dir.join("framework/sniffer_ui.js")).unwrap();
        c.eval::<(), _>(framework_code).unwrap();

        let apps_code = fs::read_to_string(dock_dir.join("services/apps.js")).unwrap();
        c.eval::<(), _>(apps_code).unwrap();

        let dock_bar_code = fs::read_to_string(dock_dir.join("components/dock_bar.js")).unwrap();
        c.eval::<(), _>(dock_bar_code).unwrap();

        let main_code = fs::read_to_string(dock_dir.join("main.js")).unwrap();
        c.eval::<(), _>(main_code).unwrap();

        fn find_element<'a>(
            el: &'a sniffer_core::types::Element,
            target_id: &str,
        ) -> Option<&'a sniffer_core::types::Element> {
            match el {
                sniffer_core::types::Element::Container { id, children, .. } => {
                    if id.as_deref() == Some(target_id) {
                        return Some(el);
                    }
                    for child in children {
                        if let Some(found) = find_element(child, target_id) {
                            return Some(found);
                        }
                    }
                    None
                }
                _ => None,
            }
        }

        let ui_port_str: String = c
            .eval("JSON.stringify(globalThis.registeredApis['dock.getUI']({ isLandscape: false }))")
            .unwrap();
        let ui_port: sniffer_core::types::Element = serde_json::from_str(&ui_port_str).unwrap();

        let port_pill_el =
            find_element(&ui_port, "dock_bar_pill").expect("dock_bar_pill in portrait");
        let port_pill_style = port_pill_el.style();
        assert_eq!(
            port_pill_style.flex_direction,
            sniffer_core::style::FlexDirection::Row
        );
        assert_eq!(
            port_pill_style.justify_content,
            sniffer_core::style::JustifyContent::SpaceAround
        );
        assert_eq!(port_pill_style.backdrop_blur, 24.0);
        assert_eq!(port_pill_style.elevation, 8.0);
        match &port_pill_style.transition.easing {
            sniffer_core::style::Easing::Spring { stiffness, damping } => {
                assert_eq!(*stiffness, 280.0);
                assert_eq!(*damping, 24.0);
            }
            other => panic!("expected Spring easing on dock_bar_pill, got {other:?}"),
        }
        let port_radius = port_pill_style.border_radius;

        let set_res_str: String = c
            .eval("JSON.stringify(globalThis.registeredApis['dock.setOrientation']({ isLandscape: true }))")
            .unwrap();
        assert!(set_res_str.contains("\"isLandscape\":true"));

        let ui_land_str: String = c
            .eval("JSON.stringify(globalThis.registeredApis['dock.getUI']({ isLandscape: true }))")
            .unwrap();
        let ui_land: sniffer_core::types::Element = serde_json::from_str(&ui_land_str).unwrap();

        let land_pill_el =
            find_element(&ui_land, "dock_bar_pill").expect("dock_bar_pill in landscape");
        let land_pill_style = land_pill_el.style();
        assert_eq!(
            land_pill_style.flex_direction,
            sniffer_core::style::FlexDirection::Column
        );
        assert_eq!(
            land_pill_style.justify_content,
            sniffer_core::style::JustifyContent::Center
        );
        assert_eq!(land_pill_style.backdrop_blur, 24.0);
        assert_eq!(land_pill_style.elevation, 8.0);
        let land_radius = land_pill_style.border_radius;
        assert_ne!(
            port_radius, land_radius,
            "portrait and landscape border radius should adapt ergonomically"
        );

        let mut manager = sniffer_core::anim::TransitionManager::default();
        manager.sync_tree(&ui_port);
        assert!(!manager.is_animating());
        assert_eq!(
            manager
                .get_current_style("dock_bar_pill")
                .unwrap()
                .border_radius,
            port_radius
        );

        manager.sync_tree(&ui_land);
        assert!(
            manager.is_animating(),
            "TransitionManager should activate upon orientation style change"
        );

        manager.tick(0.05);
        assert!(manager.is_animating());
        let mid_style = manager.get_current_style("dock_bar_pill").unwrap();
        assert_ne!(mid_style.border_radius, port_radius);
        assert_ne!(mid_style.border_radius, land_radius);

        let mut steps = 0;
        while manager.is_animating() && steps < 600 {
            manager.tick(0.016);
            steps += 1;
        }
        assert!(
            !manager.is_animating(),
            "dock spring transition must settle"
        );
        let settled_style = manager.get_current_style("dock_bar_pill").unwrap();
        assert!((settled_style.border_radius - land_radius).abs() < 1e-3);
    });
}

#[test]
fn test_dock_plugin_badge_dot_and_counter_rendering() {
    use rquickjs::{Context, Runtime};

    let plugins_dir = get_plugins_dir();
    let dock_dir = plugins_dir.join("dock");

    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();

    ctx.with(|c| {
        c.eval::<(), _>(
            r"
            globalThis.host_log = function() {};
            globalThis.host_screen_width = function() { return 1080; };
            globalThis.host_screen_height = function() { return 2400; };
            globalThis.requestPermissions = function(p) { return p; };
            globalThis.registeredApis = {};
            globalThis.registerApi = function(name, fn) { globalThis.registeredApis[name] = fn; };
            globalThis.callApi = function(name, payload) { return globalThis.registeredApis[name](payload); };
            globalThis.broadcastEvent = function() {};
            globalThis.subscribeChannel = function() {};
            globalThis.host_hash = function(str) {
                let hash = 0;
                for (let i = 0; i < str.length; i++) {
                    hash = ((hash << 5) - hash) + str.charCodeAt(i);
                    hash |= 0;
                }
                return Math.abs(hash);
            };
            globalThis.launchApp = function(pkg) {};
        ",
        )
        .unwrap();

        let framework_code =
            fs::read_to_string(plugins_dir.join("framework/sniffer_ui.js")).unwrap();
        c.eval::<(), _>(framework_code).unwrap();

        let apps_code = fs::read_to_string(dock_dir.join("services/apps.js")).unwrap();
        c.eval::<(), _>(apps_code).unwrap();

        let dock_bar_code = fs::read_to_string(dock_dir.join("components/dock_bar.js")).unwrap();
        c.eval::<(), _>(dock_bar_code).unwrap();

        let main_code = fs::read_to_string(dock_dir.join("main.js")).unwrap();
        c.eval::<(), _>(main_code).unwrap();

        fn find_element<'a>(
            el: &'a sniffer_core::types::Element,
            target_id: &str,
        ) -> Option<&'a sniffer_core::types::Element> {
            match el {
                sniffer_core::types::Element::Container { id, children, .. } => {
                    if id.as_deref() == Some(target_id) {
                        return Some(el);
                    }
                    for child in children {
                        if let Some(found) = find_element(child, target_id) {
                            return Some(found);
                        }
                    }
                    None
                }
                sniffer_core::types::Element::Label { id, .. }
                | sniffer_core::types::Element::Image { id, .. } => {
                    if id.as_deref() == Some(target_id) {
                        Some(el)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }

        let initial_ui_str: String = c
            .eval("JSON.stringify(globalThis.registeredApis['dock.getUI']({ isLandscape: false }))")
            .unwrap();
        let initial_ui: sniffer_core::types::Element =
            serde_json::from_str(&initial_ui_str).unwrap();
        assert!(find_element(&initial_ui, "dock_badge_counter_messages").is_none());
        assert!(find_element(&initial_ui, "dock_badge_dot_phone").is_none());

        c.eval::<(), _>(
            "globalThis.registeredApis['dock.setBadge']({ id: 'messages', count: 3 });",
        )
        .unwrap();
        c.eval::<(), _>(
            "globalThis.registeredApis['dock.setBadge']({ id: 'phone', dot: true });",
        )
        .unwrap();

        let badged_ui_str: String = c
            .eval("JSON.stringify(globalThis.registeredApis['dock.getUI']({ isLandscape: false }))")
            .unwrap();
        let badged_ui: sniffer_core::types::Element = serde_json::from_str(&badged_ui_str).unwrap();

        let msg_counter = find_element(&badged_ui, "dock_badge_counter_messages")
            .expect("dock_badge_counter_messages should be rendered");
        assert_eq!(msg_counter.style().background_color, Some(0xFFEF_4444));
        assert_eq!(msg_counter.style().elevation, 4.0);

        let msg_label = find_element(&badged_ui, "dock_badge_label_messages")
            .expect("dock_badge_label_messages should be rendered");
        match msg_label {
            sniffer_core::types::Element::Label { text, .. } => assert_eq!(text, "3"),
            other => panic!("expected Label element, got {other:?}"),
        }

        let phone_dot = find_element(&badged_ui, "dock_badge_dot_phone")
            .expect("dock_badge_dot_phone should be rendered");
        assert_eq!(phone_dot.style().background_color, Some(0xFFEF_4444));
        assert_eq!(phone_dot.style().elevation, 4.0);

        c.eval::<(), _>("globalThis.registeredApis['dock.clearBadge']({ id: 'messages' });")
            .unwrap();

        let cleared_ui_str: String = c
            .eval("JSON.stringify(globalThis.registeredApis['dock.getUI']({ isLandscape: false }))")
            .unwrap();
        let cleared_ui: sniffer_core::types::Element =
            serde_json::from_str(&cleared_ui_str).unwrap();
        assert!(find_element(&cleared_ui, "dock_badge_counter_messages").is_none());
        assert!(find_element(&cleared_ui, "dock_badge_dot_phone").is_some());
    });
}

#[test]
fn test_dock_plugin_long_press_preview_lifecycle() {
    use rquickjs::{Context, Runtime};

    let plugins_dir = get_plugins_dir();
    let dock_dir = plugins_dir.join("dock");

    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();

    ctx.with(|c| {
        c.eval::<(), _>(
            r"
            globalThis.host_log = function() {};
            globalThis.host_screen_width = function() { return 1080; };
            globalThis.host_screen_height = function() { return 2400; };
            globalThis.requestPermissions = function(p) { return p; };
            globalThis.registeredApis = {};
            globalThis.registerApi = function(name, fn) { globalThis.registeredApis[name] = fn; };
            globalThis.callApi = function(name, payload) { return globalThis.registeredApis[name](payload); };
            globalThis.broadcastEvent = function() {};
            globalThis.subscribeChannel = function() {};
            globalThis.host_hash = function(str) {
                let hash = 0;
                for (let i = 0; i < str.length; i++) {
                    hash = ((hash << 5) - hash) + str.charCodeAt(i);
                    hash |= 0;
                }
                return Math.abs(hash);
            };
            globalThis.launchApp = function(pkg) {};
        ",
        )
        .unwrap();

        let framework_code =
            fs::read_to_string(plugins_dir.join("framework/sniffer_ui.js")).unwrap();
        c.eval::<(), _>(framework_code).unwrap();

        let apps_code = fs::read_to_string(dock_dir.join("services/apps.js")).unwrap();
        c.eval::<(), _>(apps_code).unwrap();

        let dock_bar_code = fs::read_to_string(dock_dir.join("components/dock_bar.js")).unwrap();
        c.eval::<(), _>(dock_bar_code).unwrap();

        let main_code = fs::read_to_string(dock_dir.join("main.js")).unwrap();
        c.eval::<(), _>(main_code).unwrap();

        fn find_element<'a>(
            el: &'a sniffer_core::types::Element,
            target_id: &str,
        ) -> Option<&'a sniffer_core::types::Element> {
            match el {
                sniffer_core::types::Element::Container { id, children, .. } => {
                    if id.as_deref() == Some(target_id) {
                        return Some(el);
                    }
                    for child in children {
                        if let Some(found) = find_element(child, target_id) {
                            return Some(found);
                        }
                    }
                    None
                }
                sniffer_core::types::Element::Label { id, .. }
                | sniffer_core::types::Element::Image { id, .. } => {
                    if id.as_deref() == Some(target_id) {
                        Some(el)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }

        let initial_ui_str: String = c
            .eval("JSON.stringify(globalThis.registeredApis['dock.getUI']({ isLandscape: false }))")
            .unwrap();
        let initial_ui: sniffer_core::types::Element =
            serde_json::from_str(&initial_ui_str).unwrap();
        assert!(find_element(&initial_ui, "dock_preview_bubble").is_none());

        c.eval::<(), _>("globalThis.registeredApis['dock.triggerLongPress']({ id: 'phone' });")
            .unwrap();

        let preview_ui_str: String = c
            .eval("JSON.stringify(globalThis.registeredApis['dock.getUI']({ isLandscape: false }))")
            .unwrap();
        let preview_ui: sniffer_core::types::Element =
            serde_json::from_str(&preview_ui_str).unwrap();

        let bubble = find_element(&preview_ui, "dock_preview_bubble")
            .expect("dock_preview_bubble must be rendered on long press");
        let bubble_style = bubble.style();
        assert_eq!(bubble_style.backdrop_blur, 20.0);
        assert_eq!(bubble_style.elevation, 12.0);
        assert_eq!(bubble_style.shadow_blur, 24.0);
        match &bubble_style.transition.easing {
            sniffer_core::style::Easing::Spring { stiffness, damping } => {
                assert_eq!(*stiffness, 320.0);
                assert_eq!(*damping, 24.0);
            }
            other => panic!("expected Spring easing on preview bubble, got {other:?}"),
        }

        let title_el = find_element(&preview_ui, "dock_preview_title")
            .expect("dock_preview_title should be present in preview");
        match title_el {
            sniffer_core::types::Element::Label { text, .. } => assert_eq!(text, "Telefon"),
            other => panic!("expected Label element, got {other:?}"),
        }

        assert!(find_element(&preview_ui, "dock_preview_close_btn").is_some());

        c.eval::<(), _>("globalThis.registeredApis['dock.dismissPreview']();")
            .unwrap();

        let dismissed_ui_str: String = c
            .eval("JSON.stringify(globalThis.registeredApis['dock.getUI']({ isLandscape: false }))")
            .unwrap();
        let dismissed_ui: sniffer_core::types::Element =
            serde_json::from_str(&dismissed_ui_str).unwrap();
        assert!(find_element(&dismissed_ui, "dock_preview_bubble").is_none());
    });
}

#[test]
fn test_app_drawer_frosted_glass_backdrop_blur_and_tint_stage_2_1() {
    use rquickjs::{Context, Runtime};

    let plugins_dir = get_plugins_dir();
    let default_ui_dir = plugins_dir.join("default_ui");
    let framework_path = plugins_dir.join("framework/sniffer_ui.js");

    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();

    ctx.with(|c| {
        let setup = r"
            globalThis.host_log = function() {};
            globalThis.host_set_ui = function() {};
            globalThis.host_screen_width = function() { return 1080; };
            globalThis.host_screen_height = function() { return 2400; };
            globalThis.requestPermissions = function(p) { return p; };
            globalThis.registeredApis = {};
            globalThis.registerApi = function(name, fn) { globalThis.registeredApis[name] = fn; };
            globalThis.callApi = function(name, payload) {
                return globalThis.registeredApis[name] ? globalThis.registeredApis[name](payload) : null;
            };
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
            globalThis.SnifferUI = { forceUpdate: function() {} };
            globalThis.Vault = { queryApps: function() { return { apps: [] }; } };
            globalThis.getApplicationList = function() { return []; };
            globalThis.host_is_devkit_enabled = function() { return false; };
        ";
        c.eval::<(), _>(setup).unwrap();

        let fw_code = fs::read_to_string(&framework_path).unwrap();
        c.eval::<(), _>(fw_code).unwrap();

        let state_code = fs::read_to_string(default_ui_dir.join("state.js")).unwrap();
        c.eval::<(), _>(state_code).unwrap();

        let layout_code = fs::read_to_string(default_ui_dir.join("components/layout.js")).unwrap();
        c.eval::<(), _>(layout_code).unwrap();

        let app_grid_code =
            fs::read_to_string(default_ui_dir.join("components/app_grid.js")).unwrap();
        c.eval::<(), _>(app_grid_code).unwrap();

        let app_drawer_code =
            fs::read_to_string(default_ui_dir.join("components/app_drawer.js")).unwrap();
        c.eval::<(), _>(app_drawer_code).unwrap();

        let main_code = fs::read_to_string(default_ui_dir.join("main.js")).unwrap();
        if let Err(e) = c.eval::<(), _>(main_code.as_str()) {
            let exc = c.catch();
            panic!("eval main.js error: {e:?}, exc: {exc:?}");
        }

        fn find_element<'a>(
            el: &'a sniffer_core::types::Element,
            target_id: &str,
        ) -> Option<&'a sniffer_core::types::Element> {
            if el.id() == Some(target_id) {
                return Some(el);
            }
            match el {
                sniffer_core::types::Element::Container { children, .. }
                | sniffer_core::types::Element::ScrollView { children, .. }
                | sniffer_core::types::Element::SharedView { children, .. } => {
                    for child in children {
                        if let Some(found) = find_element(child, target_id) {
                            return Some(found);
                        }
                    }
                    None
                }
                _ => None,
            }
        }

        let drawer_ui_str: String = c.eval("JSON.stringify(AppDrawerComponent())").unwrap();
        let drawer_ui: sniffer_core::types::Element = serde_json::from_str(&drawer_ui_str).unwrap();

        let drawer_root = find_element(&drawer_ui, "app_drawer_root")
            .expect("app_drawer_root must be rendered by AppDrawerComponent");
        let style = drawer_root.style();

        assert_eq!(
            style.backdrop_blur, 32.0,
            "App drawer backdrop_blur must be 32.0"
        );
        assert_eq!(
            style.backdrop_tint,
            Some(0x880A_0E17),
            "App drawer backdrop_tint must be 0x880A0E17"
        );
        assert_eq!(
            style.background_color,
            Some(0x880A_0E17),
            "App drawer fallback background_color must be 0x880A0E17"
        );

        let root_ui_closed_str: String = c.eval("JSON.stringify(Root())").unwrap();
        let root_ui_closed: sniffer_core::types::Element =
            serde_json::from_str(&root_ui_closed_str).unwrap();
        assert!(
            find_element(&root_ui_closed, "app_drawer_root").is_none(),
            "Root UI must not contain app_drawer_root when drawer is unmounted on home screen"
        );

        c.eval::<(), _>("state.goToBottomFloor()").unwrap();
        let root_ui_open_str: String = c.eval("JSON.stringify(Root())").unwrap();
        let root_ui_open: sniffer_core::types::Element =
            serde_json::from_str(&root_ui_open_str).unwrap();
        assert!(
            find_element(&root_ui_open, "app_drawer_root").is_some(),
            "Root UI must contain app_drawer_root when drawer is mounted"
        );
    });
}

#[test]
fn test_app_drawer_staggered_spring_cascade_stage_2_2() {
    use rquickjs::{Context, Runtime};

    let plugins_dir = get_plugins_dir();
    let default_ui_dir = plugins_dir.join("default_ui");
    let framework_path = plugins_dir.join("framework/sniffer_ui.js");

    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();

    ctx.with(|c| {
        let setup = r"
            globalThis.host_log = function() {};
            globalThis.host_set_ui = function() {};
            globalThis.host_screen_width = function() { return 1080; };
            globalThis.host_screen_height = function() { return 2400; };
            globalThis.requestPermissions = function(p) { return p; };
            globalThis.registeredApis = {};
            globalThis.registerApi = function(name, fn) { globalThis.registeredApis[name] = fn; };
            globalThis.callApi = function(name, payload) {
                return globalThis.registeredApis[name] ? globalThis.registeredApis[name](payload) : null;
            };
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
            globalThis.SnifferUI = { forceUpdate: function() {} };
            globalThis.Vault = { queryApps: function() { return { apps: [] }; } };
            globalThis.getApplicationList = function() { return []; };
            globalThis.host_is_devkit_enabled = function() { return false; };
        ";
        c.eval::<(), _>(setup).unwrap();

        let fw_code = fs::read_to_string(&framework_path).unwrap();
        c.eval::<(), _>(fw_code).unwrap();

        let state_code = fs::read_to_string(default_ui_dir.join("state.js")).unwrap();
        c.eval::<(), _>(state_code).unwrap();

        let layout_code = fs::read_to_string(default_ui_dir.join("components/layout.js")).unwrap();
        c.eval::<(), _>(layout_code).unwrap();

        let app_grid_code =
            fs::read_to_string(default_ui_dir.join("components/app_grid.js")).unwrap();
        c.eval::<(), _>(app_grid_code).unwrap();

        let app_drawer_code =
            fs::read_to_string(default_ui_dir.join("components/app_drawer.js")).unwrap();
        c.eval::<(), _>(app_drawer_code).unwrap();

        let main_code = fs::read_to_string(default_ui_dir.join("main.js")).unwrap();
        c.eval::<(), _>(main_code.as_str()).unwrap();

        fn find_element<'a>(
            el: &'a sniffer_core::types::Element,
            target_id: &str,
        ) -> Option<&'a sniffer_core::types::Element> {
            if el.id() == Some(target_id) {
                return Some(el);
            }
            match el {
                sniffer_core::types::Element::Container { children, .. }
                | sniffer_core::types::Element::ScrollView { children, .. }
                | sniffer_core::types::Element::SharedView { children, .. } => {
                    for child in children {
                        if let Some(found) = find_element(child, target_id) {
                            return Some(found);
                        }
                    }
                    None
                }
                _ => None,
            }
        }

        let closed_ui_str: String = c.eval("JSON.stringify(AppDrawerComponent())").unwrap();
        let closed_ui: sniffer_core::types::Element = serde_json::from_str(&closed_ui_str).unwrap();

        let card_0_closed = find_element(&closed_ui, "drawer_card_0")
            .expect("drawer_card_0 must exist in app drawer");
        assert_eq!(card_0_closed.style().opacity, 0.0);
        assert!(card_0_closed.style().transform.translate_y > 0.0);

        assert_eq!(
            card_0_closed.style().transition.stagger_interval,
            0.015,
            "stagger_interval must be 0.015"
        );
        assert_eq!(
            card_0_closed.style().transition.stagger_index,
            0,
            "stagger_index for card 0 must be 0"
        );
        match &card_0_closed.style().transition.easing {
            sniffer_core::style::Easing::Spring { stiffness, damping } => {
                assert_eq!(*stiffness, 280.0, "Spring stiffness must be 280");
                assert_eq!(*damping, 24.0, "Spring damping must be 24");
            }
            other => panic!("expected Spring easing, got {other:?}"),
        }

        let card_1_closed = find_element(&closed_ui, "drawer_card_1")
            .expect("drawer_card_1 must exist in app drawer");
        assert_eq!(card_1_closed.style().transition.stagger_index, 1);

        let card_2_closed = find_element(&closed_ui, "drawer_card_2")
            .expect("drawer_card_2 must exist in app drawer");
        assert_eq!(card_2_closed.style().transition.stagger_index, 2);

        c.eval::<(), _>("state.goToBottomFloor();").unwrap();

        let open_ui_str: String = c.eval("JSON.stringify(AppDrawerComponent())").unwrap();
        let open_ui: sniffer_core::types::Element = serde_json::from_str(&open_ui_str).unwrap();

        let card_0_open = find_element(&open_ui, "drawer_card_0")
            .expect("drawer_card_0 must exist in open app drawer");
        assert_eq!(card_0_open.style().opacity, 1.0);
        assert_eq!(card_0_open.style().transform.translate_y, 0.0);
        assert_eq!(card_0_open.style().transition.stagger_interval, 0.015);
        assert_eq!(card_0_open.style().transition.stagger_index, 0);
    });
}

#[test]
fn test_app_drawer_rapid_open_close_timer_cancellation() {
    let plugins_dir = get_plugins_dir();
    let default_ui_dir = plugins_dir.join("default_ui");
    let framework_path = plugins_dir.join("framework/sniffer_ui.js");

    let rt = rquickjs::Runtime::new().unwrap();
    let ctx = rquickjs::Context::full(&rt).unwrap();

    ctx.with(|c| {
        let setup = r"
            globalThis.host_log = function() {};
            globalThis.host_set_ui = function() {};
            globalThis.host_screen_width = function() { return 1080; };
            globalThis.host_screen_height = function() { return 2400; };
            globalThis.requestPermissions = function(p) { return p; };
            globalThis.registeredApis = {};
            globalThis.registerApi = function(name, fn) { globalThis.registeredApis[name] = fn; };
            globalThis.callApi = function(name, payload) {
                return globalThis.registeredApis[name] ? globalThis.registeredApis[name](payload) : null;
            };
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
            globalThis.SnifferUI = { forceUpdate: function() {} };
            globalThis.Vault = { queryApps: function() { return { apps: [] }; } };
            globalThis.getApplicationList = function() { return []; };
            globalThis.host_is_devkit_enabled = function() { return false; };
            globalThis.setTimeout = function(fn) { return 1; };
            globalThis.clearTimeout = function() {};
        ";
        c.eval::<(), _>(setup).unwrap();

        let fw_code = fs::read_to_string(&framework_path).unwrap();
        c.eval::<(), _>(fw_code).unwrap();

        let state_code = fs::read_to_string(default_ui_dir.join("state.js")).unwrap();
        c.eval::<(), _>(state_code).unwrap();

        let layout_code = fs::read_to_string(default_ui_dir.join("components/layout.js")).unwrap();
        c.eval::<(), _>(layout_code).unwrap();

        let app_grid_code =
            fs::read_to_string(default_ui_dir.join("components/app_grid.js")).unwrap();
        c.eval::<(), _>(app_grid_code).unwrap();

        let app_drawer_code =
            fs::read_to_string(default_ui_dir.join("components/app_drawer.js")).unwrap();
        c.eval::<(), _>(app_drawer_code).unwrap();

        let main_code = fs::read_to_string(default_ui_dir.join("main.js")).unwrap();
        c.eval::<(), _>(main_code.as_str()).unwrap();

        fn find_element<'a>(
            el: &'a sniffer_core::types::Element,
            target_id: &str,
        ) -> Option<&'a sniffer_core::types::Element> {
            if el.id() == Some(target_id) {
                return Some(el);
            }
            match el {
                sniffer_core::types::Element::Container { children, .. }
                | sniffer_core::types::Element::ScrollView { children, .. }
                | sniffer_core::types::Element::SharedView { children, .. } => {
                    for child in children {
                        if let Some(found) = find_element(child, target_id) {
                            return Some(found);
                        }
                    }
                    None
                }
                _ => None,
            }
        }

        let is_mounted: bool = c.eval("state.isDrawerMounted").unwrap();
        assert!(!is_mounted, "Drawer must start unmounted on home floor");

        c.eval::<(), _>("state.goToBottomFloor()").unwrap();
        let is_mounted: bool = c.eval("state.isDrawerMounted").unwrap();
        let floor: String = c.eval("state.currentFloor").unwrap();
        assert!(is_mounted, "Drawer must be mounted after goToBottomFloor");
        assert_eq!(floor, "bottom");

        c.eval::<(), _>("state.goToTopFloor()").unwrap();
        let is_mounted: bool = c.eval("state.isDrawerMounted").unwrap();
        let floor: String = c.eval("state.currentFloor").unwrap();
        let has_timer: bool = c.eval("state._drawerCloseTimer !== null").unwrap();
        assert!(
            is_mounted,
            "Drawer must stay mounted during closing animation"
        );
        assert_eq!(floor, "top");
        assert!(has_timer, "Close timer must be active");

        c.eval::<(), _>("state.goToBottomFloor()").unwrap();
        let is_mounted: bool = c.eval("state.isDrawerMounted").unwrap();
        let floor: String = c.eval("state.currentFloor").unwrap();
        let timer_cleared: bool = c.eval("state._drawerCloseTimer === null").unwrap();
        assert!(is_mounted, "Drawer must remain mounted on rapid reopen");
        assert_eq!(floor, "bottom");
        assert!(timer_cleared, "Close timer must be cancelled on reopen");

        c.eval::<(), _>("state.goToTopFloor()").unwrap();
        c.eval::<(), _>(
            r#"
            if (state.currentFloor === "top") {
                state.isDrawerMounted = false;
            }
        "#,
        )
        .unwrap();
        let is_mounted_final: bool = c.eval("state.isDrawerMounted").unwrap();
        assert!(
            !is_mounted_final,
            "Drawer must be unmounted after timer fires"
        );

        let root_ui_str: String = c.eval("JSON.stringify(Root())").unwrap();
        let root_ui: sniffer_core::types::Element = serde_json::from_str(&root_ui_str).unwrap();
        assert!(find_element(&root_ui, "app_drawer_root").is_none());
    });
}
