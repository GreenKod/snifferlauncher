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
    #[serde(default)]
    pub preload: Vec<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
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

    /// Validate a manifest and return a list of human-readable issues.
    #[must_use]
    pub fn validate_manifest(manifest: &PluginManifest) -> Vec<String> {
        let mut issues = Vec::new();

        if manifest.id.trim().is_empty() {
            issues.push("manifest id must not be empty".to_string());
        }
        if manifest.name.trim().is_empty() {
            issues.push("manifest name must not be empty".to_string());
        }
        if manifest.version.trim().is_empty() {
            issues.push("manifest version must not be empty".to_string());
        }
        if manifest.main.trim().is_empty() {
            issues.push("manifest main entry must not be empty".to_string());
        }
        if manifest.permissions.iter().any(|p| p.trim().is_empty()) {
            issues.push("manifest permissions must not contain empty values".to_string());
        }

        issues
    }

    /// Instantiate and register all plugins defined in `plugins.json`.
    #[allow(clippy::too_many_lines)]
    pub fn register_all(
        &self,
        registry: &mut PluginRegistry,
        action_queue: &Arc<std::sync::Mutex<Vec<crate::core::types::Action>>>,
    ) {
        if !self.assets_dir.exists() {
            println!(
                "Plugin directory {} does not exist.",
                self.assets_dir.display()
            );
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

        let api_map = registry.api_registry();
        let broadcast_queue = registry.broadcast_queue();

        for plugin_folder in config.active_plugins {
            let plugin_dir = self.assets_dir.join(&plugin_folder);
            let manifest_path = plugin_dir.join("manifest.json");

            if !manifest_path.exists() {
                eprintln!("manifest.json not found for plugin '{plugin_folder}'");
                continue;
            }

            let manifest_str = match fs::read_to_string(&manifest_path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Failed to read manifest.json for plugin '{plugin_folder}': {e}");
                    continue;
                }
            };

            let manifest: PluginManifest = match serde_json::from_str(&manifest_str) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("Failed to parse manifest.json for plugin '{plugin_folder}': {e}");
                    continue;
                }
            };

            let manifest_issues = Self::validate_manifest(&manifest);
            if !manifest_issues.is_empty() {
                eprintln!(
                    "Manifest validation failed for plugin '{}': {}",
                    plugin_folder,
                    manifest_issues.join(", ")
                );
                continue;
            }

            // Collect preload scripts (e.g. framework JS) before the main plugin code.
            let mut preload_scripts: Vec<String> = Vec::new();
            for preload_path in &manifest.preload {
                // Paths are relative to the .plugins root (e.g. "../_framework/sniffer_ui.js")
                let resolved = plugin_dir.join(preload_path);
                match fs::read_to_string(&resolved) {
                    Ok(src) => {
                        println!(
                            "Preloading '{}' for plugin '{}'",
                            preload_path, manifest.name
                        );
                        preload_scripts.push(src);
                    }
                    Err(e) => eprintln!(
                        "Could not read preload '{}' for plugin '{}': {e}",
                        preload_path, manifest.name
                    ),
                }
            }

            let main_js_path = plugin_dir.join(&manifest.main);
            if main_js_path.exists() {
                match fs::read_to_string(&main_js_path) {
                    Ok(main_content) => {
                        // Concatenate preload scripts + main script into one bundle.
                        let mut full_script = preload_scripts.join("\n");
                        if !full_script.is_empty() {
                            full_script.push('\n');
                        }
                        full_script.push_str(&main_content);

                        let hash = crate::core::ui::widget::fnv1a(full_script.as_bytes());
                        let cache_dir = self.assets_dir.join(".cache");
                        let _ = fs::create_dir_all(&cache_dir);
                        let cache_file = cache_dir.join(format!("{}_{}_ui.bin", manifest.id, hash));

                        let mut cached_ui = None;
                        if cache_file.exists() {
                            println!(
                                "Cache file found for {}: {}",
                                manifest.id,
                                cache_file.display()
                            );
                            if let Ok(bytes) = fs::read(&cache_file) {
                                if let Ok(ui) =
                                    postcard::from_bytes::<crate::core::types::Element>(&bytes)
                                {
                                    println!(
                                        "Successfully deserialized UI from cache for {}",
                                        manifest.id
                                    );
                                    cached_ui = Some(ui);
                                } else {
                                    println!("Failed to deserialize postcard for {}", manifest.id);
                                }
                            }
                        } else {
                            println!(
                                "No cache file found for {}. Will be created upon host_set_ui.",
                                manifest.id
                            );
                        }

                        match crate::plugin::JsPlugin::new(crate::plugin::js::JsPluginConfig {
                            script_content: full_script,
                            plugin_id: manifest.id.clone(),
                            action_queue: action_queue.clone(),
                            api_map: api_map.clone(),
                            broadcast_queue: broadcast_queue.clone(),
                            permissions: manifest.permissions.clone(),
                            cached_ui,
                            cache_path: Some(cache_file),
                        }) {
                            Ok(plugin) => {
                                registry.register(
                                    &(Arc::new(plugin) as Arc<dyn crate::plugin::UiPlugin>),
                                );
                                println!(
                                    "Successfully loaded JS plugin '{}' ({}) from {}",
                                    manifest.name,
                                    manifest.id,
                                    main_js_path.display()
                                );
                            }
                            Err(e) => {
                                eprintln!(
                                    "Failed to instantiate JS plugin '{}': {e}",
                                    manifest.name
                                );
                            }
                        }
                    }
                    Err(e) => eprintln!(
                        "Could not read {} for plugin '{}': {e}",
                        manifest.main, manifest.name
                    ),
                }
            } else {
                eprintln!(
                    "Main script '{}' not found for plugin '{}'",
                    manifest.main, manifest.name
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PluginLoader, PluginManifest};

    #[test]
    fn validate_manifest_accepts_basic_plugin_manifest() {
        let manifest = PluginManifest {
            id: "com.example.plugin".to_string(),
            name: "Example Plugin".to_string(),
            version: "1.0.0".to_string(),
            main: "main.js".to_string(),
            preload: vec![],
            permissions: vec!["android.permission.CAMERA".to_string()],
        };

        let issues = PluginLoader::validate_manifest(&manifest);
        assert!(issues.is_empty());
    }

    #[test]
    fn validate_manifest_reports_missing_required_fields() {
        let manifest = PluginManifest {
            id: "   ".to_string(),
            name: String::new(),
            version: " ".to_string(),
            main: String::new(),
            preload: vec![],
            permissions: vec![String::new()],
        };

        let issues = PluginLoader::validate_manifest(&manifest);
        assert!(issues.iter().any(|issue| issue.contains("id")));
        assert!(issues.iter().any(|issue| issue.contains("name")));
        assert!(issues.iter().any(|issue| issue.contains("version")));
        assert!(issues.iter().any(|issue| issue.contains("main")));
        assert!(issues.iter().any(|issue| issue.contains("permission")));
    }
}

#[cfg(target_os = "android")]
impl PluginLoader {
    /// Read plugins from Android Assets instead of the filesystem.
    ///
    /// # Panics
    /// Instantiate and register all plugins defined in `plugins.json` from Android assets.
    #[allow(clippy::too_many_lines)]
    pub fn register_all_from_assets(
        registry: &mut PluginRegistry,
        asset_manager: &ndk::asset::AssetManager,
        action_queue: &Arc<std::sync::Mutex<Vec<crate::core::types::Action>>>,
    ) {
        use std::io::Read;
        let config_cstr = std::ffi::CString::new("plugins.json").unwrap();

        let config_string = if let Some(mut asset) = asset_manager.open(config_cstr.as_c_str()) {
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

        let config: PluginsConfig = match serde_json::from_str(&config_string) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to parse plugins.json on Android: {e}");
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
                            // Collect preload scripts from Android assets.
                            let mut preload_scripts: Vec<String> = Vec::new();
                            for preload_rel in &manifest.preload {
                                // Resolve the path relative to the plugin folder:
                                // e.g. "../_framework/sniffer_ui.js" -> "_framework/sniffer_ui.js"
                                let resolved = std::path::Path::new(&plugin_folder)
                                    .join(preload_rel)
                                    .to_string_lossy()
                                    .replace('\\', "/");
                                // Normalise: remove leading "./" or "../" components naively
                                let resolved = resolved.trim_start_matches("../").to_string();
                                if let Ok(cstr) = std::ffi::CString::new(resolved.clone()) {
                                    if let Some(mut pa) = asset_manager.open(cstr.as_c_str()) {
                                        let mut src = String::new();
                                        if pa.read_to_string(&mut src).is_ok() {
                                            println!(
                                                "Android: preloading '{}' for plugin '{}'",
                                                resolved, manifest.name
                                            );
                                            preload_scripts.push(src);
                                        }
                                    } else {
                                        eprintln!("Android: preload asset '{resolved}' not found");
                                    }
                                }
                            }

                            let main_js_path = format!("{plugin_folder}/{}", manifest.main);
                            if let Ok(main_cstr) = std::ffi::CString::new(main_js_path.clone()) {
                                if let Some(mut main_asset) =
                                    asset_manager.open(main_cstr.as_c_str())
                                {
                                    let mut main_content = String::new();
                                    if main_asset.read_to_string(&mut main_content).is_ok() {
                                        // Bundle preload + main
                                        let mut full_script = preload_scripts.join("\n");
                                        if !full_script.is_empty() {
                                            full_script.push('\n');
                                        }
                                        full_script.push_str(&main_content);

                                        match crate::plugin::JsPlugin::new(
                                            full_script,
                                            manifest.id.clone(),
                                            action_queue.clone(),
                                            api_map.clone(),
                                            broadcast_queue.clone(),
                                            manifest.permissions.clone(),
                                            None,
                                            None,
                                        ) {
                                            Ok(plugin) => {
                                                registry.register(
                                                    &(Arc::new(plugin)
                                                        as Arc<dyn crate::plugin::UiPlugin>),
                                                );
                                                println!(
                                                    "Successfully loaded JS plugin '{}' ({}) from Android Assets",
                                                    manifest.name, manifest.id
                                                );
                                            }
                                            Err(e) => eprintln!(
                                                "Failed to instantiate JS plugin '{}' on Android: {e}",
                                                manifest.name
                                            ),
                                        }
                                    } else {
                                        eprintln!(
                                            "Failed to read content of {main_js_path} from Android assets"
                                        );
                                    }
                                } else {
                                    eprintln!(
                                        "Main script '{}' not found for plugin '{}' in Android assets",
                                        manifest.main, manifest.name
                                    );
                                }
                            }
                        } else {
                            eprintln!(
                                "Failed to parse manifest.json for plugin '{plugin_folder}' on Android"
                            );
                        }
                    } else {
                        eprintln!(
                            "Failed to read manifest.json for plugin '{plugin_folder}' on Android"
                        );
                    }
                } else {
                    eprintln!("manifest.json not found for plugin '{plugin_folder}' on Android");
                }
            }
        }
    }
}
