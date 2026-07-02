pub mod js_plugin;
pub mod loader;
pub mod registry;
pub mod r#trait;

pub use js_plugin::JsPlugin;
pub use loader::PluginLoader;
pub use registry::PluginRegistry;
pub use r#trait::UiPlugin;
