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
}
