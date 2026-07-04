use crate::core::ui::data_map::DataMap;
use crate::core::ui::event::UiEvent;
use crate::core::ui::style_map::StyleMap;
use crate::core::ui::widget::WidgetId;

/// Interface that external plugins must implement.
///
/// When a plugin is registered, the system reads `subscriptions()` and
/// inserts `ID -> [this_plugin]` entries into `PluginRegistry`'s internal `HashMap`.
/// When an event arrives, only the listeners for that specific ID are invoked.
///
/// # Thread Safety
/// `Send + Sync` is required; plugins are called from the render thread
/// but may be shared across other threads.
pub trait UiPlugin: Send + Sync {
    /// The widget IDs this plugin wants to observe.
    ///
    /// Returns a slice — no heap allocation, zero cost.
    fn subscriptions(&self) -> &[WidgetId];

    /// Called when a relevant `UiEvent` is received for a subscribed ID.
    ///
    /// The plugin may update `StyleMap` or `DataMap` to change the appearance
    /// or content of widgets without touching `app.rs`.
    fn on_event(
        &self,
        event: &UiEvent,
        styles: &StyleMap,
        data: &DataMap,
        actions: &std::sync::Arc<std::sync::Mutex<Vec<crate::core::types::Action>>>,
    );

    /// Ask the plugin to provide the initial UI layout tree.
    ///
    /// If multiple plugins provide a layout, the system may use the first one or merge them.
    /// Default implementation returns `None`, indicating the plugin doesn't define layout.
    fn build_ui(&self) -> Option<crate::core::types::Element> {
        None
    }

    /// Called once per render frame.
    /// Useful for periodic tasks, animations, or garbage collection.
    fn on_tick(&self) {}

    /// Called when another plugin broadcasts a message to all plugins via `broadcastEvent`.
    ///
    /// # Arguments
    /// * `channel` — The broadcast channel name (e.g. `"store.changed"`).
    /// * `payload_json` — A JSON string containing the broadcast data.
    fn on_broadcast(&self, _channel: &str, _payload_json: &str) {}
}
