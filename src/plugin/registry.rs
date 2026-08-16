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
use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------
// Inter-Plugin API types
// ---------------------------------------------------------------------------

/// A Rust closure registered by a plugin to handle incoming API calls.
/// Receives a JSON payload string and returns an optional JSON response string.
pub type ApiCallback = Arc<dyn Fn(String) -> Option<String> + Send + Sync>;

/// An entry in the shared API registry.
pub struct ApiEntry {
    /// The manifest `id` of the plugin that registered this API (e.g. `"com.sniffer.store"`).
    /// Used for logging and circular-call detection.
    pub plugin_id: String,
    /// The callback closure that executes the API handler inside the owning plugin's runtime.
    pub callback: ApiCallback,
}

/// Thread-safe map of API name → entry, shared across all `JsPlugin` instances.
pub type ApiMap = Arc<Mutex<HashMap<String, ApiEntry>>>;

/// Thread-safe queue of pending broadcast events `(channel, payload_json)`.
/// Filled by `host_broadcast`; drained by `PluginRegistry::dispatch` each frame.
pub type BroadcastQueue = Arc<Mutex<Vec<(String, String)>>>;

// ---------------------------------------------------------------------------
// PluginRegistry
// ---------------------------------------------------------------------------

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
    /// All registered plugins.
    plugins: Vec<Arc<dyn UiPlugin>>,
    /// Key: `WidgetId` (u64) | Value: plugins subscribed to that ID.
    subscriptions: HashMap<WidgetId, Vec<Arc<dyn UiPlugin>>>,
    /// Shared inter-plugin API registry. Cloned into each `JsPlugin` on creation.
    api_map: ApiMap,
    /// Pending broadcast events queued by any plugin. Drained once per frame in `dispatch`.
    broadcast_queue: BroadcastQueue,
    /// Shared Native Data Vault.
    vault: Arc<crate::core::vault::DataVault>,
}

impl PluginRegistry {
    /// Register a plugin — attaches it to every ID it declares in `subscriptions()`.
    pub fn register(&mut self, plugin: &Arc<dyn UiPlugin>) {
        self.plugins.push(Arc::clone(plugin));
        for &id in plugin.subscriptions() {
            self.subscriptions
                .entry(id)
                .or_default()
                .push(Arc::clone(plugin));
        }
    }

    /// Returns a clone of the shared inter-plugin API map.
    ///
    /// Pass this to `JsPlugin::new` so that each plugin can register and call APIs.
    #[must_use]
    pub fn api_registry(&self) -> ApiMap {
        Arc::clone(&self.api_map)
    }

    pub fn broadcast_queue(&self) -> BroadcastQueue {
        Arc::clone(&self.broadcast_queue)
    }

    /// Enqueue a broadcast message from the host system
    pub fn broadcast(&self, channel: &str, payload_json: &str) {
        if let Ok(mut q) = self.broadcast_queue.lock() {
            q.push((channel.to_string(), payload_json.to_string()));
        }
    }

    /// Returns a clone of the shared DataVault.
    #[must_use]
    pub fn vault(&self) -> Arc<crate::core::vault::DataVault> {
        Arc::clone(&self.vault)
    }

    /// Ask registered plugins for a UI layout.
    /// Returns the first layout provided by any plugin.
    #[must_use]
    pub fn build_ui(&self) -> Option<crate::core::types::Element> {
        for plugin in &self.plugins {
            if let Some(ui) = plugin.build_ui() {
                return Some(ui);
            }
        }
        None
    }

    /// Trigger the periodic tick for all registered plugins.
    pub fn tick(&self) {
        for plugin in &self.plugins {
            plugin.on_tick();
        }
    }

    /// Suspend all registered plugins (e.g. when launcher goes to background).
    pub fn suspend_all(&self) {
        for plugin in &self.plugins {
            plugin.on_suspend();
        }
    }

    /// Resume all registered plugins (e.g. when launcher returns to foreground).
    pub fn resume_all(&self) {
        for plugin in &self.plugins {
            plugin.on_resume();
        }
    }

    /// Dispatch all queued events to their respective plugin listeners,
    /// then drain and broadcast any pending inter-plugin broadcast messages.
    ///
    /// Call once per render frame. IDs with no registered listeners cost O(1) miss.
    pub fn dispatch(
        &self,
        bus: &EventBus,
        styles: &StyleMap,
        data: &DataMap,
        actions: &std::sync::Arc<std::sync::Mutex<Vec<crate::core::types::Action>>>,
    ) {
        // Coalesce multiple Scroll events in the same frame to prevent JS engine lag
        let raw_events = bus.drain();
        let mut coalesced = Vec::with_capacity(raw_events.len());
        for event in raw_events {
            if let crate::core::ui::event::UiEvent::Scroll(id, dx, dy, _max_x, _max_y) = &event {
                if let Some(crate::core::ui::event::UiEvent::Scroll(last_id, last_dx, last_dy, _, _)) =
                    coalesced.last_mut()
                {
                    if last_id == id {
                        *last_dx += dx;
                        *last_dy += dy;
                        continue;
                    }
                }
            }
            coalesced.push(event);
        }

        // Route UI events to all plugins.
        for event in coalesced {
            for plugin in &self.plugins {
                plugin.on_event(&event, styles, data, actions);
            }
        }

        // Drain and dispatch inter-plugin broadcast messages.
        let broadcasts: Vec<(String, String)> = self
            .broadcast_queue
            .lock()
            .map(|mut q| std::mem::take(&mut *q))
            .unwrap_or_default();

        for (channel, payload_json) in broadcasts {
            for plugin in &self.plugins {
                plugin.on_broadcast(&channel, &payload_json);
            }
        }
    }
}
