pub mod hover_effect;
pub mod loader;
pub mod registry;
pub mod r#trait;
pub mod wasm_plugin;

pub use hover_effect::HoverEffectPlugin;
pub use loader::PluginLoader;
pub use registry::PluginRegistry;
pub use r#trait::UiPlugin;
pub use wasm_plugin::WasmPlugin;
