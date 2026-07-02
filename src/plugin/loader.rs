use crate::plugin::registry::PluginRegistry;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Loads and registers JavaScript plugins from the assets directory.
pub struct PluginLoader {
    assets_dir: PathBuf,
}

impl PluginLoader {
    /// Create a new loader pointing to the specified directory (e.g., `assets/ui`).
    #[must_use]
    pub fn new(assets_dir: impl AsRef<Path>) -> Self {
        Self {
            assets_dir: assets_dir.as_ref().to_path_buf(),
        }
    }

    /// Instantiate and register all plugins.
    pub fn register_all(&self, registry: &mut PluginRegistry) {
        let main_js_path = self.assets_dir.join("main.js");

        match fs::read_to_string(&main_js_path) {
            Ok(content) => match crate::plugin::JsPlugin::new(content) {
                Ok(plugin) => {
                    registry.register(&(Arc::new(plugin) as Arc<dyn crate::plugin::UiPlugin>));
                    println!(
                        "Successfully loaded JS plugin from {}",
                        main_js_path.display()
                    );
                }
                Err(e) => eprintln!("Failed to instantiate JS plugin: {e}"),
            },
            Err(e) => eprintln!("Could not read main.js at {}: {e}", main_js_path.display()),
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
    ) {
        use std::io::Read;
        let cstr = std::ffi::CString::new("ui/main.js").unwrap();
        if let Some(mut asset) = asset_manager.open(cstr.as_c_str()) {
            let mut content = String::new();
            if asset.read_to_string(&mut content).is_ok() {
                match crate::plugin::JsPlugin::new(content) {
                    Ok(plugin) => {
                        registry.register(&(Arc::new(plugin) as Arc<dyn crate::plugin::UiPlugin>));
                        println!("Successfully loaded JS plugin from Android Assets");
                    }
                    Err(e) => eprintln!("Failed to instantiate JS plugin on Android: {e}"),
                }
            } else {
                eprintln!("Failed to read content of ui/main.js from assets");
            }
        } else {
            eprintln!("Failed to open ui/main.js from Android Assets");
        }
    }
}
