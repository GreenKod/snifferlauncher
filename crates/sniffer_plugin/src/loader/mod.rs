pub mod android_loader;
pub mod fs_loader;
pub mod manifest;
#[cfg(test)]
pub mod tests;

pub use manifest::{PluginManifest, PluginsConfig};

use std::path::{Path, PathBuf};

/// Loads and registers JavaScript plugins from the assets directory.
pub struct PluginLoader {
    pub(crate) assets_dir: PathBuf,
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
}
