#![allow(clippy::pedantic, clippy::nursery)]

pub mod engine;
pub mod host_api;

use crate::core::types::Element;
use crate::core::ui::data_map::DataMap;
use crate::core::ui::event::UiEvent;
use crate::core::ui::style_map::StyleMap;
use crate::core::ui::widget::WidgetId;
use crate::plugin::registry::{ApiMap, BroadcastQueue};
use crate::plugin::r#trait::UiPlugin;
use rquickjs::{Context, Runtime, Value};
use std::sync::{Arc, Mutex};

use engine::create_engine;
use host_api::{HostApiConfig, register_host_api};

// ---------------------------------------------------------------------------
// JS preamble injected into every plugin context before the plugin code runs.
// This defines the public API surface for inter-plugin communication.
// ---------------------------------------------------------------------------
const IPC_PREAMBLE: &str = r#"
// --- SnifferLauncher Inter-Plugin API ---

// Internal map: API name -> JS handler function
globalThis._apis = {};

/**
 * Register a named API endpoint for this plugin.
 * Other plugins can call it by name using callApi().
 *
 * @param {string} name - Unique dot-namespaced name, e.g. "store.get"
 * @param {function} handler - (payload: object) => any
 */
globalThis.registerApi = function(name, handler) {
    globalThis._apis[name] = handler;
    host_register_api(name);
};

/**
 * Call a named API exposed by any registered plugin (synchronous).
 *
 * @param {string} name - API name, e.g. "store.get"
 * @param {object} payload - JSON-serializable payload
 * @returns {object|null} - JSON-parsed response or null on failure/missing API
 */
globalThis.callApi = function(name, payload) {
    const json = host_call_api(name, JSON.stringify(payload ?? {}));
    if (json === null || json === undefined) return null;
    try { return JSON.parse(json); } catch(e) { return null; }
};

/**
 * Broadcast an event to ALL registered plugins (delivered next frame).
 *
 * @param {string} channel - Channel name, e.g. "theme.changed"
 * @param {object} data - JSON-serializable data
 */
globalThis.broadcastEvent = function(channel, data) {
    host_broadcast(channel, JSON.stringify(data ?? {}));
};

/**
 * Override this in your plugin to receive broadcasts from other plugins.
 *
 * @param {string} channel
 * @param {object} data
 */
globalThis.onBroadcast = function(channel, data) {};

// Internal — Rust calls this to deliver a broadcast to this plugin.
globalThis._dispatchBroadcast = function(channel, payload_json) {
    try {
        const data = JSON.parse(payload_json);
        globalThis.onBroadcast(channel, data);
    } catch(e) {}
};

// Internal — Rust calls this when another plugin invokes one of our APIs.
globalThis._handleApiCall = function(name, payload_json) {
    if (!globalThis._apis[name]) return null;
    try {
        const payload = JSON.parse(payload_json);
        const result = globalThis._apis[name](payload);
        return JSON.stringify(result ?? null);
    } catch(e) {
        return null;
    }
};
"#;

// ---------------------------------------------------------------------------
// JsPlugin
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub struct JsPlugin {
    script_content: String,
    runtime: Runtime,
    context: Context,
    subscriptions: Vec<WidgetId>,
    ui_tree: Arc<Mutex<Option<Element>>>,
    gc_counter: std::sync::Mutex<u32>,
    action_queue: Arc<Mutex<Vec<crate::core::types::Action>>>,
    api_map: ApiMap,
    broadcast_queue: BroadcastQueue,
}

#[allow(clippy::non_send_fields_in_send_ty)]
unsafe impl Send for JsPlugin {}
unsafe impl Sync for JsPlugin {}

impl JsPlugin {
    /// Initialize a QuickJS runtime and context for a plugin.
    ///
    /// # Errors
    /// Returns a `String` error if runtime/context creation or script evaluation fails.
    pub fn new(
        script_content: String,
        plugin_id: String,
        action_queue: Arc<Mutex<Vec<crate::core::types::Action>>>,
        api_map: ApiMap,
        broadcast_queue: BroadcastQueue,
        cached_ui: Option<Element>,
        cache_path: Option<std::path::PathBuf>,
    ) -> Result<Self, String> {
        let (runtime, context) = create_engine()?;
        let ui_tree = Arc::new(Mutex::new(cached_ui));

        let plugin = Self {
            script_content,
            runtime,
            context,
            subscriptions: vec![],
            ui_tree: ui_tree.clone(),
            gc_counter: std::sync::Mutex::new(0),
            action_queue: action_queue.clone(),
            api_map: api_map.clone(),
            broadcast_queue: broadcast_queue.clone(),
        };

        plugin.init_js_env(
            plugin_id,
            ui_tree,
            action_queue,
            api_map,
            broadcast_queue,
            cache_path,
        )?;

        Ok(plugin)
    }

    fn init_js_env(
        &self,
        plugin_id: String,
        ui_tree: Arc<Mutex<Option<Element>>>,
        action_queue: Arc<Mutex<Vec<crate::core::types::Action>>>,
        api_map: ApiMap,
        broadcast_queue: BroadcastQueue,
        cache_path: Option<std::path::PathBuf>,
    ) -> Result<(), String> {
        // Clone the context to pass into host_api (captured as SafeContext).
        let context_clone = self.context.clone();

        self.context.with(|ctx| {
            register_host_api(
                &ctx,
                HostApiConfig {
                    plugin_id,
                    context: context_clone,
                    ui_tree,
                    action_queue,
                    api_map,
                    broadcast_queue,
                    cache_path,
                },
            );

            // Inject the IPC preamble before the plugin's own code.
            let _ = ctx
                .eval::<Value, _>(IPC_PREAMBLE.as_bytes())
                .map_err(|e| e.to_string())?;

            // Evaluate the plugin script.
            let _ = ctx
                .eval::<Value, _>(self.script_content.as_bytes())
                .map_err(|e| e.to_string())?;

            Ok::<(), String>(())
        })?;

        Ok(())
    }
}

impl UiPlugin for JsPlugin {
    fn subscriptions(&self) -> &[WidgetId] {
        &[]
    }

    fn build_ui(&self) -> Option<Element> {
        if let Ok(lock) = self.ui_tree.lock() {
            return lock.clone();
        }
        None
    }

    fn on_event(
        &self,
        event: &UiEvent,
        _styles: &StyleMap,
        _data: &DataMap,
        _actions: &Arc<Mutex<Vec<crate::core::types::Action>>>,
    ) {
        self.context.with(|ctx| {
            let globals = ctx.globals();
            if let Ok(on_event_fn) = globals.get::<_, rquickjs::Function>("onEvent") {
                let event_json = match event {
                    UiEvent::Click(id, w, h) => {
                        format!(r#"{{"type":"Click","id":"{id}","w":{w},"h":{h}}}"#)
                    }
                    UiEvent::Hover(id, w, h, x, y) => {
                        format!(r#"{{"type":"Hover","id":"{id}","w":{w},"h":{h},"x":{x},"y":{y}}}"#)
                    }
                    UiEvent::HoverEnd(id) => format!(r#"{{"type":"HoverEnd","id":"{id}"}}"#),
                    UiEvent::TextInput(text) => format!(
                        r#"{{"type":"TextInput","text":{}}}"#,
                        serde_json::to_string(text).unwrap_or_default()
                    ),
                    UiEvent::ClickOutside => r#"{"type":"ClickOutside"}"#.to_string(),
                    UiEvent::Backspace => r#"{"type":"Backspace"}"#.to_string(),
                    UiEvent::PointerDown(id) => format!(r#"{{"type":"PointerDown","id":"{id}"}}"#),
                    UiEvent::PointerUp(id_opt) => {
                        let id_str = match id_opt {
                            Some(id) => format!(r#""{id}""#),
                            None => "null".to_string(),
                        };
                        format!(r#"{{"type":"PointerUp","id":{id_str}}}"#)
                    }
                    UiEvent::Scroll(id_opt, dx, dy, max_y) => {
                        let id_str = match id_opt {
                            Some(id) => format!(r#""{id}""#),
                            None => "null".to_string(),
                        };
                        format!(r#"{{"type":"Scroll","id":{id_str},"dx":{dx},"dy":{dy},"max_y":{max_y}}}"#)
                    }
                    UiEvent::WindowResized(w, h) => {
                        format!(r#"{{"type":"WindowResized","w":{w},"h":{h}}}"#)
                    }
                };

                let res: Result<String, _> = on_event_fn.call((event_json,));
                if let Ok(_cmds_json) = res {
                    // Expect JS to return an array of DrawCommands or Actions in JSON
                }
            }
        });
    }

    fn on_tick(&self) {
        if let Ok(mut count) = self.gc_counter.lock() {
            *count += 1;
            if *count >= 300 {
                *count = 0;
                self.runtime.run_gc();
            }
        }
    }

    /// Deliver a broadcast from another plugin to this plugin's `onBroadcast` handler.
    fn on_broadcast(&self, channel: &str, payload_json: &str) {
        self.context.with(|ctx| {
            if let Ok(handler) = ctx
                .globals()
                .get::<_, rquickjs::Function>("_dispatchBroadcast")
            {
                let _ = handler.call::<_, ()>((channel.to_string(), payload_json.to_string()));
            }
        });
    }
}
