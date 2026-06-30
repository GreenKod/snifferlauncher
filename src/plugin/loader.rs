use crate::plugin::HoverEffectPlugin;
use crate::plugin::registry::PluginRegistry;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Plugin type: either a native (compiled-in) Rust plugin or a Wasm sandbox plugin.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginType {
    Native,
    Wasm,
}

/// Minimal location block — only used when `plugin_type` is `wasm`.
#[derive(Debug, Serialize, Deserialize)]
pub struct PluginLocation {
    /// Relative path from `.plugins/` to the compiled `.wasm` file.
    /// Example: `"com.greenkod.wasm-sample/wasm_sample.wasm"`
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PluginIdentifier {
    pub id: String,
    pub uuid: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PluginMetadata {
    #[serde(rename = "isBuiltin", default)]
    pub is_builtin: bool,
    #[serde(rename = "publisherDisplayName")]
    pub publisher_display_name: String,
}

/// A single entry in `plugins.json`.
#[derive(Debug, Serialize, Deserialize)]
pub struct PluginEntry {
    pub identifier: PluginIdentifier,
    pub version: String,
    #[serde(rename = "type")]
    pub plugin_type: PluginType,
    /// True when the plugin source lives in `.plugins/<source_dir>/`.
    /// The build script compiles it; the output `.wasm` lands in `location.path`.
    pub has_source: bool,
    /// Present only when `has_source` is true. Folder name inside `.plugins/`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_dir: Option<String>,
    /// Present only for `type = "wasm"` plugins.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<PluginLocation>,
    pub metadata: PluginMetadata,
}

/// Loads and registers plugins declared in `.plugins/plugins.json`.
pub struct PluginLoader {
    plugins_dir: PathBuf,
}

impl PluginLoader {
    /// Create a new loader pointing to the specified directory (e.g., `.plugins`).
    #[must_use]
    pub fn new(plugins_dir: impl AsRef<Path>) -> Self {
        Self {
            plugins_dir: plugins_dir.as_ref().to_path_buf(),
        }
    }

    /// Read the `plugins.json` file and parse the entries.
    ///
    /// # Errors
    /// Returns an empty list if the file doesn't exist or cannot be parsed.
    #[must_use]
    pub fn load_manifest(&self) -> Vec<PluginEntry> {
        let json_path = self.plugins_dir.join("plugins.json");
        match fs::read_to_string(&json_path) {
            Ok(content) => match serde_json::from_str::<Vec<PluginEntry>>(&content) {
                Ok(entries) => entries,
                Err(e) => {
                    eprintln!("Failed to parse plugins.json: {e}");
                    Vec::new()
                }
            },
            Err(e) => {
                eprintln!(
                    "Could not read plugins.json at {}: {e}",
                    json_path.display()
                );
                Vec::new()
            }
        }
    }

    /// Instantiate and register all plugins declared in the manifest.
    pub fn register_all(&self, registry: &mut PluginRegistry) {
        for entry in self.load_manifest() {
            println!("Loading plugin: {} v{}", entry.identifier.id, entry.version);

            match entry.plugin_type {
                PluginType::Wasm => {
                    let Some(loc) = &entry.location else {
                        eprintln!("Wasm plugin {} has no location field", entry.identifier.id);
                        continue;
                    };
                    let wasm_path = self.plugins_dir.join(&loc.path);
                    match fs::read(&wasm_path) {
                        Ok(bytes) => match crate::plugin::WasmPlugin::new(&bytes) {
                            Ok(plugin) => {
                                registry.register(
                                    &(Arc::new(plugin) as Arc<dyn crate::plugin::UiPlugin>),
                                );
                                println!(
                                    "Successfully loaded Wasm plugin: {}",
                                    entry.identifier.id
                                );
                            }
                            Err(e) => eprintln!(
                                "Failed to instantiate Wasm plugin {}: {e}",
                                entry.identifier.id
                            ),
                        },
                        Err(e) => eprintln!("Could not read .wasm at {}: {e}", wasm_path.display()),
                    }
                }
                PluginType::Native => match entry.identifier.id.as_str() {
                    "com.greenkod.hover-effect" => {
                        registry.register(
                            &(Arc::new(HoverEffectPlugin) as Arc<dyn crate::plugin::UiPlugin>),
                        );
                    }
                    id => eprintln!("Unknown native plugin ID: {id}"),
                },
            }
        }
    }
}
