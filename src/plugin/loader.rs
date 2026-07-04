use crate::plugin::registry::PluginRegistry;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct PluginsConfig {
    pub active_plugins: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub main: String,
}

/// Loads and registers JavaScript plugins from the assets directory.
pub struct PluginLoader {
    assets_dir: PathBuf,
}

impl PluginLoader {
    /// Create a new loader pointing to the specified directory (e.g., `.plugins`).
    #[must_use]
    pub fn new(assets_dir: impl AsRef<Path>) -> Self {
        Self {
            assets_dir: assets_dir.as_ref().to_path_buf(),
        }
    }

    /// Instantiate and register all plugins defined in plugins.json.
    pub fn register_all(
        &self,
        registry: &mut PluginRegistry,
        action_queue: Arc<std::sync::Mutex<Vec<crate::core::types::Action>>>,
    ) {
        if !self.assets_dir.exists() {
            println!("Plugin directory {} does not exist.", self.assets_dir.display());
            return;
        }

        let plugins_json_path = self.assets_dir.join("plugins.json");
        if !plugins_json_path.exists() {
            println!("plugins.json not found in {}", self.assets_dir.display());
            return;
        }

        let config_str = match fs::read_to_string(&plugins_json_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to read plugins.json: {e}");
                return;
            }
        };

        let config: PluginsConfig = match serde_json::from_str(&config_str) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to parse plugins.json: {e}");
                return;
            }
        };

        for plugin_folder in config.active_plugins {
            let plugin_dir = self.assets_dir.join(&plugin_folder);
            let manifest_path = plugin_dir.join("manifest.json");

            if !manifest_path.exists() {
                eprintln!("manifest.json not found for plugin '{}'", plugin_folder);
                continue;
            }

            let manifest_str = match fs::read_to_string(&manifest_path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Failed to read manifest.json for plugin '{}': {e}", plugin_folder);
                    continue;
                }
            };

            let manifest: PluginManifest = match serde_json::from_str(&manifest_str) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("Failed to parse manifest.json for plugin '{}': {e}", plugin_folder);
                    continue;
                }
            };

            let main_js_path = plugin_dir.join(&manifest.main);
            if main_js_path.exists() {
                match fs::read_to_string(&main_js_path) {
                    Ok(content) => match crate::plugin::JsPlugin::new(content, action_queue.clone()) {
                        Ok(plugin) => {
                            registry.register(&(Arc::new(plugin) as Arc<dyn crate::plugin::UiPlugin>));
                            println!("Successfully loaded JS plugin '{}' ({}) from {}", manifest.name, manifest.id, main_js_path.display());
                        }
                        Err(e) => eprintln!("Failed to instantiate JS plugin '{}': {e}", manifest.name),
                    },
                    Err(e) => eprintln!("Could not read {} for plugin '{}': {e}", manifest.main, manifest.name),
                }
            } else {
                eprintln!("Main script '{}' not found for plugin '{}'", manifest.main, manifest.name);
            }
        }
    }
}

#[cfg(target_os = "android")]
impl PluginLoader {
    /// Read plugins from Android Assets instead of the filesystem.
    ///
    /// # Panics
    /// Panics if the internal asset path string contains a null byte.
    pub fn register_all_from_assets(
        registry: &mut PluginRegistry,
        asset_manager: &ndk::asset::AssetManager,
        action_queue: Arc<std::sync::Mutex<Vec<crate::core::types::Action>>>,
    ) {
        use std::io::Read;
        let config_cstr = std::ffi::CString::new("plugins.json").unwrap();
        
        let config_str = if let Some(mut asset) = asset_manager.open(config_cstr.as_c_str()) {
            let mut content = String::new();
            if asset.read_to_string(&mut content).is_ok() {
                content
            } else {
                eprintln!("Failed to read plugins.json from assets");
                return;
            }
        } else {
            eprintln!("plugins.json not found in Android assets");
            return;
        };

        let config: PluginsConfig = match serde_json::from_str(&config_str) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to parse plugins.json on Android: {e}");
                return;
            }
        };

        for plugin_folder in config.active_plugins {
            let manifest_path = format!("{}/manifest.json", plugin_folder);
            if let Ok(manifest_cstr) = std::ffi::CString::new(manifest_path.clone()) {
                if let Some(mut asset) = asset_manager.open(manifest_cstr.as_c_str()) {
                    let mut manifest_str = String::new();
                    if asset.read_to_string(&mut manifest_str).is_ok() {
                        if let Ok(manifest) = serde_json::from_str::<PluginManifest>(&manifest_str) {
                            let main_js_path = format!("{}/{}", plugin_folder, manifest.main);
                            if let Ok(main_cstr) = std::ffi::CString::new(main_js_path.clone()) {
                                if let Some(mut main_asset) = asset_manager.open(main_cstr.as_c_str()) {
                                    let mut content = String::new();
                                    if main_asset.read_to_string(&mut content).is_ok() {
                                        match crate::plugin::JsPlugin::new(content, action_queue.clone()) {
                                            Ok(plugin) => {
                                                registry.register(&(Arc::new(plugin) as Arc<dyn crate::plugin::UiPlugin>));
                                                println!("Successfully loaded JS plugin '{}' ({}) from Android Assets", manifest.name, manifest.id);
                                            }
                                            Err(e) => eprintln!("Failed to instantiate JS plugin '{}' on Android: {e}", manifest.name),
                                        }
                                    } else {
                                        eprintln!("Failed to read content of {} from Android assets", main_js_path);
                                    }
                                } else {
                                    eprintln!("Main script '{}' not found for plugin '{}' in Android assets", manifest.main, manifest.name);
                                }
                            }
                        } else {
                            eprintln!("Failed to parse manifest.json for plugin '{}' on Android", plugin_folder);
                        }
                    } else {
                        eprintln!("Failed to read manifest.json for plugin '{}' on Android", plugin_folder);
                    }
                } else {
                    eprintln!("manifest.json not found for plugin '{}' on Android", plugin_folder);
                }
            }
        }
    }
}
