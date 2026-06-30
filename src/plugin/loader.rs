use crate::plugin::HoverEffectPlugin;
use crate::plugin::registry::PluginRegistry;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// A struct reflecting the VS Code style `extensions.json` structure
#[derive(Debug, Serialize, Deserialize)]
pub struct ExtensionIdentifier {
    pub id: String,
    pub uuid: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExtensionLocation {
    #[serde(rename = "$mid")]
    pub mid: u32,
    pub path: String,
    pub scheme: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExtensionMetadata {
    pub source: String,
    pub id: String,
    #[serde(rename = "publisherId")]
    pub publisher_id: String,
    #[serde(rename = "publisherDisplayName")]
    pub publisher_display_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PluginEntry {
    pub identifier: ExtensionIdentifier,
    pub version: String,
    pub location: ExtensionLocation,
    #[serde(rename = "relativeLocation")]
    pub relative_location: String,
    pub metadata: ExtensionMetadata,
}

/// Loads plugins from a `plugins.json` file in a specified directory.
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
    /// Returns an empty list if the file doesn't exist or parsing fails.
    #[must_use]
    pub fn load_manifest(&self) -> Vec<PluginEntry> {
        let json_path = self.plugins_dir.join("plugins.json");
        if let Ok(content) = fs::read_to_string(&json_path) {
            if let Ok(entries) = serde_json::from_str::<Vec<PluginEntry>>(&content) {
                return entries;
            } else {
                eprintln!("Failed to parse plugins.json");
            }
        } else {
            eprintln!("Could not read plugins.json at {:?}", json_path);
        }
        Vec::new()
    }

    /// Instantiates and registers the native (or in the future, Wasm) plugins
    /// based on the loaded manifest.
    pub fn register_all(&self, registry: &mut PluginRegistry) {
        let entries = self.load_manifest();
        for entry in entries {
            println!("Loading plugin: {} v{}", entry.identifier.id, entry.version);

            // For now, since we only have static plugins, we route by ID.
            // In the future (Phase 4), this will compile & load the Wasm module from `entry.location.path`.
            match entry.identifier.id.as_str() {
                "com.greenkod.hover-effect" => {
                    let plugin = Arc::new(HoverEffectPlugin) as Arc<dyn crate::plugin::UiPlugin>;
                    registry.register(&plugin);
                }
                _ => {
                    eprintln!("Unknown plugin ID: {}", entry.identifier.id);
                }
            }
        }
    }
}
