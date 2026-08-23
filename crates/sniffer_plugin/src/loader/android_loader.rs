#[cfg(target_os = "android")]
use super::PluginLoader;
#[cfg(target_os = "android")]
use super::manifest::{PluginManifest, PluginsConfig};
#[cfg(target_os = "android")]
use crate::registry::PluginRegistry;
#[cfg(target_os = "android")]
use crate::{dev_err, dev_log};
#[cfg(target_os = "android")]
use obfstr::obfstr;
#[cfg(target_os = "android")]
use std::sync::Arc;

#[cfg(target_os = "android")]
impl PluginLoader {
    /// Read plugins from Android Assets instead of the filesystem.
    #[allow(clippy::too_many_lines)]
    pub fn register_all_from_assets(
        registry: &mut PluginRegistry,
        asset_manager: &ndk::asset::AssetManager,
        action_queue: &Arc<std::sync::Mutex<Vec<sniffer_core::types::Action>>>,
    ) {
        use std::io::Read;
        let config_cstr = std::ffi::CString::new(obfstr!("plugins.json")).unwrap();

        let config_string = if let Some(mut asset) = asset_manager.open(config_cstr.as_c_str()) {
            let mut content = String::new();
            if asset.read_to_string(&mut content).is_ok() {
                content
            } else {
                dev_err!("{}", obfstr!("Failed to read plugins.json from assets"));
                return;
            }
        } else {
            dev_err!("{}", obfstr!("plugins.json not found in Android assets"));
            return;
        };

        let config: PluginsConfig = match serde_json::from_str(&config_string) {
            Ok(c) => c,
            Err(e) => {
                dev_err!(
                    "{}: {e}",
                    obfstr!("Failed to parse plugins.json on Android")
                );
                return;
            }
        };

        let api_map = registry.api_registry();
        let broadcast_queue = registry.broadcast_queue();

        for plugin_folder in config.active_plugins {
            let manifest_path = format!("{plugin_folder}/manifest.json");
            if let Ok(manifest_cstr) = std::ffi::CString::new(manifest_path.clone()) {
                if let Some(mut asset) = asset_manager.open(manifest_cstr.as_c_str()) {
                    let mut manifest_str = String::new();
                    if asset.read_to_string(&mut manifest_str).is_ok() {
                        if let Ok(manifest) = serde_json::from_str::<PluginManifest>(&manifest_str)
                        {
                            let mut preload_scripts: Vec<String> = Vec::new();
                            for preload_rel in &manifest.preload {
                                let resolved =
                                    if let Some(stripped) = preload_rel.strip_prefix("../") {
                                        stripped.to_string()
                                    } else {
                                        format!("{plugin_folder}/{preload_rel}")
                                    };
                                let alt_resolved = resolved.replace("_framework/", "framework/");
                                let asset_opt = std::ffi::CString::new(resolved.clone())
                                    .ok()
                                    .and_then(|c| asset_manager.open(c.as_c_str()))
                                    .or_else(|| {
                                        std::ffi::CString::new(alt_resolved)
                                            .ok()
                                            .and_then(|c| asset_manager.open(c.as_c_str()))
                                    });

                                if let Some(mut pa) = asset_opt {
                                    let mut src = String::new();
                                    if pa.read_to_string(&mut src).is_ok() {
                                        let clean_src = src.replace('\0', "").trim().to_string();
                                        dev_log!(
                                            "{} '{}' {} '{}'",
                                            obfstr!("Android: preloading"),
                                            resolved,
                                            obfstr!("for plugin"),
                                            manifest.name
                                        );
                                        preload_scripts.push(clean_src);
                                    }
                                } else {
                                    dev_err!(
                                        "{} '{resolved}' {}",
                                        obfstr!("Android: preload asset"),
                                        obfstr!("not found")
                                    );
                                }
                            }

                            let files_to_read = if !manifest.scripts.is_empty() {
                                manifest.scripts.clone()
                            } else {
                                vec![manifest.main.clone()]
                            };

                            let mut plugin_code = String::new();
                            for script_rel in &files_to_read {
                                let script_path = format!("{plugin_folder}/{script_rel}");
                                if let Ok(script_cstr) = std::ffi::CString::new(script_path) {
                                    if let Some(mut asset) =
                                        asset_manager.open(script_cstr.as_c_str())
                                    {
                                        let mut content = String::new();
                                        if asset.read_to_string(&mut content).is_ok() {
                                            let clean_content =
                                                content.replace('\0', "").trim().to_string();
                                            if !plugin_code.is_empty() {
                                                plugin_code.push('\n');
                                            }
                                            plugin_code.push_str(&clean_content);
                                        }
                                    }
                                }
                            }

                            if !plugin_code.is_empty() {
                                let mut full_script = preload_scripts.join("\n");
                                if !full_script.is_empty() {
                                    full_script.push('\n');
                                }
                                full_script.push_str(&plugin_code);
                                let full_script = full_script.replace('\0', "");
                                let full_script = full_script.trim_matches('\0').trim().to_string();

                                match crate::JsPlugin::new(crate::js::JsPluginConfig {
                                    script_content: full_script,
                                    plugin_id: manifest.id.clone(),
                                    vault: registry.vault(),
                                    action_queue: action_queue.clone(),
                                    api_map: api_map.clone(),
                                    broadcast_queue: broadcast_queue.clone(),
                                    permissions: manifest.permissions.clone(),
                                    default_settings: manifest.default_settings.clone(),
                                    cached_ui: None,
                                    cache_path: None,
                                    pkg_registry: registry.pkg_registry(),
                                }) {
                                    Ok(plugin) => {
                                        registry.register(
                                            &(Arc::new(plugin) as Arc<dyn crate::UiPlugin>),
                                        );
                                        dev_log!(
                                            "{} '{}' ({}) {}",
                                            obfstr!("Successfully loaded JS plugin"),
                                            manifest.name,
                                            manifest.id,
                                            obfstr!("from Android Assets")
                                        );
                                    }
                                    Err(e) => dev_err!(
                                        "{} '{}' {}: {e}",
                                        obfstr!("Failed to instantiate JS plugin"),
                                        manifest.name,
                                        obfstr!("on Android")
                                    ),
                                }
                            } else {
                                dev_err!(
                                    "{} '{}' {}",
                                    obfstr!("Failed to read scripts for plugin"),
                                    manifest.name,
                                    obfstr!("from Android assets")
                                );
                            }
                        } else {
                            dev_err!(
                                "{} '{plugin_folder}' {}",
                                obfstr!("Failed to parse manifest.json for plugin"),
                                obfstr!("on Android")
                            );
                        }
                    } else {
                        dev_err!(
                            "{} '{plugin_folder}' {}",
                            obfstr!("Failed to read manifest.json for plugin"),
                            obfstr!("on Android")
                        );
                    }
                }
            }
        }
    }
}
