//! Plugin Registry and Dispatch System
//!
//! The Plugin Registry is the heart of the modular UI architecture. Instead of hardcoding
//! application logic (like hover effects, clicks, and navigation) directly into the
//! core platform code, the `PluginRegistry` allows isolated components (Plugins) to subscribe
//! to specific widget IDs.
//!
//! Plugins can be either Native (compiled Rust) or WebAssembly (sandboxed modules loaded at runtime).
//! When an event occurs, the Registry efficiently routes the event only to the plugins
//! that explicitly subscribed to the target widget, guaranteeing O(1) event dispatching
//! for non-interactive elements.

use crate::core::ui::data_map::DataMap;
use crate::core::ui::event::EventBus;
use crate::core::ui::style_map::StyleMap;
use crate::core::ui::widget::WidgetId;
use crate::plugin::r#trait::UiPlugin;
use std::collections::HashMap;
use std::sync::Arc;

/// Manages a collection of plugins and efficiently routes events to them.
///
/// Registration complexity  : `O(subscriptions.len())` amortised
/// Dispatch complexity      : `O(1)` `HashMap` lookup + `O(k)` dispatch
///   where `k` = number of plugins subscribed to a given ID (typically 1–3)
///
/// # Example
/// ```ignore
/// let mut registry = PluginRegistry::default();
/// registry.register(&Arc::new(HoverEffectPlugin));
/// // Each render frame:
/// registry.dispatch(&bus, &styles, &data);
/// ```
#[derive(Default)]
pub struct PluginRegistry {
    /// Key: `WidgetId` (u64) | Value: plugins subscribed to that ID.
    subscriptions: HashMap<WidgetId, Vec<Arc<dyn UiPlugin>>>,
}

impl PluginRegistry {
    /// Register a plugin — attaches it to every ID it declares in `subscriptions()`.
    pub fn register(&mut self, plugin: &Arc<dyn UiPlugin>) {
        for &id in plugin.subscriptions() {
            self.subscriptions
                .entry(id)
                .or_default()
                .push(Arc::clone(plugin));
        }
    }

    /// Dispatch all queued events to their respective plugin listeners.
    ///
    /// Call once per render frame. IDs with no registered listeners cost O(1) miss.
    pub fn dispatch(
        &self,
        bus: &EventBus,
        styles: &StyleMap,
        data: &DataMap,
        actions: &std::sync::Arc<std::sync::Mutex<Vec<crate::core::types::Action>>>,
    ) {
        for event in bus.drain() {
            let id = event.widget_id();
            if let Some(listeners) = self.subscriptions.get(&id) {
                for plugin in listeners {
                    plugin.on_event(&event, styles, data, actions);
                }
            }
        }
    }
}
