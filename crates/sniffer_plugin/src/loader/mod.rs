pub mod manifest;

pub use manifest::{PluginManifest, PluginsConfig};

use crate::registry::PluginRegistry;
use crate::{dev_err, dev_log};
use obfstr::obfstr;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

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
        manifest.validate()
    }

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

            let manifest: PluginManifest = match serde_json::from_str(&manifest_str) {
                Ok(m) => m,
                Err(e) => {
                    dev_err!(
                        "{} '{plugin_folder}': {e}",
                        obfstr!("Failed to parse manifest.json for plugin")
                    );
                    continue;
                }
            };

            let manifest_issues = Self::validate_manifest(&manifest);
            if !manifest_issues.is_empty() {
                dev_err!(
                    "{} '{}': {}",
                    obfstr!("Manifest validation failed for plugin"),
                    plugin_folder,
                    manifest_issues.join(", ")
                );
                continue;
            }

            // Collect preload scripts (e.g. framework JS) before the main plugin code.
            let mut preload_scripts: Vec<String> = Vec::new();
            for preload_path in &manifest.preload {
                let resolved = plugin_dir.join(preload_path);
                match fs::read_to_string(&resolved) {
                    Ok(src) => {
                        dev_log!(
                            "{} '{}' {} '{}'",
                            obfstr!("Preloading"),
                            preload_path,
                            obfstr!("for plugin"),
                            manifest.name
                        );
                        preload_scripts.push(src);
                    }
                    Err(e) => dev_err!(
                        "{} '{}' {} '{}': {e}",
                        obfstr!("Could not read preload"),
                        preload_path,
                        obfstr!("for plugin"),
                        manifest.name
                    ),
                }
            }

            let files_to_read = if !manifest.scripts.is_empty() {
                manifest.scripts.clone()
            } else {
                vec![manifest.main.clone()]
            };

            let mut plugin_code = String::new();
            for script_rel_path in &files_to_read {
                let js_path = plugin_dir.join(script_rel_path);
                if js_path.exists() {
                    match fs::read_to_string(&js_path) {
                        Ok(content) => {
                            if !plugin_code.is_empty() {
                                plugin_code.push('\n');
                            }
                            plugin_code.push_str(&content);
                        }
                        Err(e) => dev_err!(
                            "{} '{script_rel_path}' {} '{}': {e}",
                            obfstr!("Could not read script"),
                            obfstr!("for plugin"),
                            manifest.name
                        ),
                    }
                }
            }

            if !plugin_code.is_empty() {
                let mut full_script = preload_scripts.join("\n");
                if !full_script.is_empty() {
                    full_script.push('\n');
                }
                full_script.push_str(&plugin_code);

                let hash = sniffer_core::ui::widget::fnv1a(full_script.as_bytes());
                let cache_dir = self.assets_dir.join(obfstr!(".cache"));
                let _ = fs::create_dir_all(&cache_dir);
                let cache_file = cache_dir.join(format!("{}_{}_ui.bin", manifest.id, hash));

                let mut cached_ui = None;
                if cache_file.exists() {
                    dev_log!(
                        "{} {}: {}",
                        obfstr!("Cache file found for"),
                        manifest.id,
                        cache_file.display()
                    );
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
}
