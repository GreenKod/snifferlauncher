use super::PluginLoader;
use super::manifest::PluginsConfig;
use crate::registry::PluginRegistry;
use crate::{dev_err, dev_log};
use obfstr::obfstr;
use std::fs;
use std::sync::Arc;

impl PluginLoader {
    /// Instantiate and register all plugins defined in `plugins.json`.
    #[allow(clippy::too_many_lines)]
    pub fn register_all(
        &self,
        registry: &mut PluginRegistry,
        action_queue: &Arc<std::sync::Mutex<Vec<sniffer_core::types::Action>>>,
    ) {
        if !self.assets_dir.exists() {
            dev_log!(
                "{} {}",
                obfstr!("Plugin directory does not exist:"),
                self.assets_dir.display()
            );
            return;
        }

        let plugins_json_path = self.assets_dir.join(obfstr!("plugins.json"));
        if !plugins_json_path.exists() {
            dev_log!(
                "{} {}",
                obfstr!("plugins.json not found in"),
                self.assets_dir.display()
            );
            return;
        }

        let config_str = match fs::read_to_string(&plugins_json_path) {
            Ok(s) => s,
            Err(e) => {
                dev_err!("{}: {e}", obfstr!("Failed to read plugins.json"));
                return;
            }
        };

        let config: PluginsConfig = match serde_json::from_str(&config_str) {
            Ok(c) => c,
            Err(e) => {
                dev_err!("{}: {e}", obfstr!("Failed to parse plugins.json"));
                return;
            }
        };

        if let Some(ref master) = config.master_plugin {
            dev_log!(
                "{} '{master}'",
                obfstr!("[PluginLoader] Master plugin designated:")
            );
        }

        let api_map = registry.api_registry();
        let broadcast_queue = registry.broadcast_queue();

        for plugin_folder in config.active_plugins {
            let plugin_dir = self.assets_dir.join(&plugin_folder);
            let manifest_path = plugin_dir.join(obfstr!("manifest.json"));

            if !manifest_path.exists() {
                dev_err!(
                    "{} '{plugin_folder}'",
                    obfstr!("manifest.json not found for plugin")
                );
                continue;
            }

            let manifest_str = match fs::read_to_string(&manifest_path) {
                Ok(s) => s,
                Err(e) => {
                    dev_err!(
                        "{} '{plugin_folder}': {e}",
                        obfstr!("Failed to read manifest.json for plugin")
                    );
                    continue;
                }
            };

            let manifest: super::manifest::PluginManifest =
                match serde_json::from_str(&manifest_str) {
                    Ok(m) => m,
                    Err(e) => {
                        dev_err!(
                            "{} '{plugin_folder}': {e}",
                            obfstr!("Failed to parse manifest.json for plugin")
                        );
                        continue;
                    }
                };

            let issues = manifest.validate();
            if !issues.is_empty() {
                dev_err!(
                    "{} '{}': {:?}",
                    obfstr!("Manifest validation issues for plugin"),
                    manifest.name,
                    issues
                );
            }

            let mut preload_scripts: Vec<String> = Vec::new();
            for preload_rel in &manifest.preload {
                let preload_path = if let Some(stripped) = preload_rel.strip_prefix("../") {
                    self.assets_dir.join(stripped)
                } else {
                    plugin_dir.join(preload_rel)
                };

                let preload_path = if !preload_path.exists() {
                    let fallback_str = preload_path
                        .to_string_lossy()
                        .replace("_framework\\", "framework\\")
                        .replace("_framework/", "framework/");
                    std::path::PathBuf::from(fallback_str)
                } else {
                    preload_path
                };

                match fs::read_to_string(&preload_path) {
                    Ok(src) => {
                        let clean_src = src.replace('\0', "").trim().to_string();
                        dev_log!(
                            "{} '{}' {} '{}'",
                            obfstr!("Preloading"),
                            preload_path.display(),
                            obfstr!("for plugin"),
                            manifest.name
                        );
                        preload_scripts.push(clean_src);
                    }
                    Err(e) => {
                        dev_err!(
                            "{} '{}': {e}",
                            obfstr!("Failed to read preload script"),
                            preload_path.display()
                        );
                    }
                }
            }

            let files_to_read = if !manifest.scripts.is_empty() {
                manifest.scripts.clone()
            } else {
                vec![manifest.main.clone()]
            };

            let mut plugin_code = String::new();
            for script_rel in &files_to_read {
                let script_path = plugin_dir.join(script_rel);
                match fs::read_to_string(&script_path) {
                    Ok(content) => {
                        let clean_content = content.replace('\0', "").trim().to_string();
                        if !plugin_code.is_empty() {
                            plugin_code.push('\n');
                        }
                        plugin_code.push_str(&clean_content);
                    }
                    Err(e) => {
                        dev_err!(
                            "{} '{}': {e}",
                            obfstr!("Failed to read script"),
                            script_path.display()
                        );
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

                let cache_dir = self.assets_dir.join(obfstr!(".cache"));
                if !cache_dir.exists() {
                    let _ = fs::create_dir_all(&cache_dir);
                }
                let cache_file = cache_dir.join(format!("{}.ui.bin", manifest.id));

                let mut cached_ui: Option<sniffer_core::types::Element> = None;
                if cache_file.exists() {
                    dev_log!("{}: {}", obfstr!("Found cached UI file for"), manifest.id);
                    if let Ok(bytes) = fs::read(&cache_file) {
                        if let Ok(ui) = postcard::from_bytes::<sniffer_core::types::Element>(&bytes)
                        {
                            dev_log!(
                                "{}: {}",
                                obfstr!("Successfully deserialized UI from cache for"),
                                manifest.id
                            );
                            cached_ui = Some(ui);
                        } else {
                            dev_log!(
                                "{} {}",
                                obfstr!("Failed to deserialize postcard for"),
                                manifest.id
                            );
                        }
                    }
                } else {
                    dev_log!(
                        "{} {}. {}",
                        obfstr!("No cache file found for"),
                        manifest.id,
                        obfstr!("Will be created upon host_set_ui.")
                    );
                }

                match crate::JsPlugin::new(crate::js::JsPluginConfig {
                    script_content: full_script,
                    plugin_id: manifest.id.clone(),
                    vault: registry.vault(),
                    action_queue: action_queue.clone(),
                    api_map: api_map.clone(),
                    broadcast_queue: broadcast_queue.clone(),
                    permissions: manifest.permissions.clone(),
                    default_settings: manifest.default_settings.clone(),
                    cached_ui,
                    cache_path: Some(cache_file),
                    pkg_registry: registry.pkg_registry(),
                }) {
                    Ok(plugin) => {
                        registry.register(&(Arc::new(plugin) as Arc<dyn crate::UiPlugin>));
                        dev_log!(
                            "{} '{}' ({})",
                            obfstr!("Successfully loaded JS plugin"),
                            manifest.name,
                            manifest.id
                        );
                    }
                    Err(e) => {
                        dev_err!(
                            "{} '{}': {e}",
                            obfstr!("Failed to instantiate JS plugin"),
                            manifest.name
                        );
                    }
                }
            } else {
                dev_err!(
                    "{} '{}'",
                    obfstr!("No script content found for plugin"),
                    manifest.name
                );
            }
        }
    }
}
