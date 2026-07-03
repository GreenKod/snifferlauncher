#![allow(clippy::pedantic, clippy::nursery)]

use crate::core::types::Element;
use crate::core::ui::data_map::DataMap;
use crate::core::ui::event::UiEvent;
use crate::core::ui::style_map::StyleMap;
use crate::core::ui::widget::WidgetId;
use crate::plugin::r#trait::UiPlugin;
use rquickjs::{Context, Function, Runtime, Value};
use std::sync::{Arc, Mutex};

#[allow(dead_code)]
pub struct JsPlugin {
    script_content: String,
    runtime: Runtime,
    context: Context,
    subscriptions: Vec<WidgetId>,
    ui_tree: Arc<Mutex<Option<Element>>>,
    gc_counter: std::sync::Mutex<u32>,
    action_queue: Arc<Mutex<Vec<crate::core::types::Action>>>,
}

#[allow(clippy::non_send_fields_in_send_ty)]
unsafe impl Send for JsPlugin {}
unsafe impl Sync for JsPlugin {}

impl JsPlugin {
    /// Initialize QuickJS runtime and context.
    ///
    /// # Errors
    /// Returns a String error if QuickJS runtime or context creation fails.
    pub fn new(
        script_content: String,
        action_queue: Arc<Mutex<Vec<crate::core::types::Action>>>,
    ) -> Result<Self, String> {
        let runtime = Runtime::new().map_err(|e| format!("QuickJS runtime error: {e}"))?;
        let context = Context::full(&runtime).map_err(|e| format!("QuickJS context error: {e}"))?;

        let ui_tree = Arc::new(Mutex::new(None));

        let plugin = Self {
            script_content,
            runtime,
            context,
            subscriptions: vec![], // For now, we subscribe to everything (or nothing specifically),
            // but let's just make it a global event listener.
            ui_tree: ui_tree.clone(),
            gc_counter: std::sync::Mutex::new(0),
            action_queue,
        };

        plugin.init_js_env(ui_tree)?;

        Ok(plugin)
    }

    fn init_js_env(&self, ui_tree: Arc<Mutex<Option<Element>>>) -> Result<(), String> {
        self.context.with(|ctx| {
            let globals = ctx.globals();

            // host_set_ui
            let tree_set_ui = ui_tree.clone();
            let set_ui_func =
                Function::new(
                    ctx.clone(),
                    move |json_str: String| match serde_json::from_str::<Element>(&json_str) {
                        Ok(parsed) => {
                            if let Ok(mut lock) = tree_set_ui.lock() {
                                *lock = Some(parsed);
                            }
                        }
                        Err(e) => {
                            println!("JS Error: Failed to parse host_set_ui JSON: {e}");
                        }
                    },
                )
                .unwrap();
            globals.set("host_set_ui", set_ui_func).unwrap();

            // host_update_style
            let tree_update_style = ui_tree.clone();
            let update_style_func = Function::new(
                ctx.clone(),
                move |id: String, property: String, value: String| {
                    if let Ok(mut lock) = tree_update_style.lock()
                        && let Some(ref mut root) = *lock
                    {
                        root.mutate_style(&id, &property, &value);
                    }
                },
            )
            .unwrap();
            globals.set("host_update_style", update_style_func).unwrap();

            // host_set_text
            let tree_set_text = ui_tree.clone();
            let set_text_func = Function::new(ctx.clone(), move |id: String, text: String| {
                if let Ok(mut lock) = tree_set_text.lock()
                    && let Some(ref mut root) = *lock
                {
                    root.mutate_text(&id, &text);
                }
            })
            .unwrap();
            globals.set("host_set_text", set_text_func).unwrap();

            // host_insert_child
            let tree_insert_child = ui_tree.clone();
            let insert_child_func =
                Function::new(ctx.clone(), move |parent_id: String, child_json: String| {
                    if let Ok(parsed_child) = serde_json::from_str::<Element>(&child_json) {
                        if let Ok(mut lock) = tree_insert_child.lock()
                            && let Some(ref mut root) = *lock
                        {
                            root.insert_child(&parent_id, parsed_child);
                        }
                    } else {
                        println!("JS Error: Failed to parse child JSON in host_insert_child");
                    }
                })
                .unwrap();
            globals.set("host_insert_child", insert_child_func).unwrap();

            // host_remove_node
            let tree_remove_node = ui_tree.clone();
            let remove_node_func = Function::new(ctx.clone(), move |id: String| {
                if let Ok(mut lock) = tree_remove_node.lock()
                    && let Some(ref mut root) = *lock
                {
                    root.remove_node(&id);
                }
            })
            .unwrap();
            globals.set("host_remove_node", remove_node_func).unwrap();

            // host_get_binary_state (Phase 2)
            fn get_binary_state<'js>(
                ctx: rquickjs::Ctx<'js>,
            ) -> rquickjs::Result<rquickjs::ArrayBuffer<'js>> {
                let state = crate::core::types::AppState {
                    click_count: 42, // Dummy count for example
                    screen_width: f32::from_bits(
                        crate::core::types::SCREEN_WIDTH.load(std::sync::atomic::Ordering::Relaxed),
                    ),
                    screen_height: f32::from_bits(
                        crate::core::types::SCREEN_HEIGHT
                            .load(std::sync::atomic::Ordering::Relaxed),
                    ),
                };
                let bytes = postcard::to_allocvec(&state).unwrap_or_default();
                rquickjs::ArrayBuffer::new(ctx, bytes)
            }
            let get_binary_state_func = Function::new(ctx.clone(), get_binary_state).unwrap();
            globals
                .set("host_get_binary_state", get_binary_state_func)
                .unwrap();

            // host_send_binary_event (Phase 2)
            let send_binary_event_func =
                Function::new(ctx.clone(), |buffer: rquickjs::ArrayBuffer<'_>| {
                    if let Some(bytes) = buffer.as_bytes() {
                        if let Ok(state) =
                            postcard::from_bytes::<crate::core::types::AppState>(bytes)
                        {
                            println!("JS sent binary state via ArrayBuffer: {:?}", state);
                        } else {
                            println!("Failed to deserialize binary event from JS.");
                        }
                    }
                })
                .unwrap();
            globals
                .set("host_send_binary_event", send_binary_event_func)
                .unwrap();

            // host_log
            let log_func = Function::new(ctx.clone(), |msg: String| {
                println!("JS Log: {msg}");
            })
            .unwrap();
            globals.set("host_log", log_func).unwrap();

            // host_hash (converts string to WidgetId hash as string)
            let hash_func = Function::new(ctx.clone(), |s: String| -> String {
                crate::core::ui::widget::fnv1a(s.as_bytes()).to_string()
            })
            .unwrap();
            globals.set("host_hash", hash_func).unwrap();

            // host_screen_width
            let get_width_func = Function::new(ctx.clone(), || -> f32 {
                f32::from_bits(
                    crate::core::types::SCREEN_WIDTH.load(std::sync::atomic::Ordering::Relaxed),
                )
            })
            .unwrap();
            globals.set("host_screen_width", get_width_func).unwrap();

            // host_screen_height
            let get_height_func = Function::new(ctx.clone(), || -> f32 {
                f32::from_bits(
                    crate::core::types::SCREEN_HEIGHT.load(std::sync::atomic::Ordering::Relaxed),
                )
            })
            .unwrap();
            globals.set("host_screen_height", get_height_func).unwrap();

            // host_create_image
            let aq_img = self.action_queue.clone();
            let create_image_func = Function::new(ctx.clone(), move |id: String, src: String| {
                if let Ok(mut q) = aq_img.lock() {
                    q.push(crate::core::types::Action::LoadImage { id, src });
                }
            })
            .unwrap();
            globals.set("host_create_image", create_image_func).unwrap();

            // host_focus_input
            let aq_focus = self.action_queue.clone();
            let focus_input_func = Function::new(ctx.clone(), move |id: String| {
                if let Ok(mut q) = aq_focus.lock() {
                    q.push(crate::core::types::Action::FocusTextInput(id));
                }
            })
            .unwrap();
            globals.set("host_focus_input", focus_input_func).unwrap();

            // host_blur_input
            let aq_blur = self.action_queue.clone();
            let blur_input_func = Function::new(ctx.clone(), move || {
                if let Ok(mut q) = aq_blur.lock() {
                    q.push(crate::core::types::Action::BlurTextInput);
                }
            })
            .unwrap();
            globals.set("host_blur_input", blur_input_func).unwrap();

            // evaluate script
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
        // Return a static empty slice or a predefined list if needed.
        // For simplicity, we can let the registry dispatch all events to this plugin,
        // or we just return an empty array and handle events via global dispatch.
        // Wait, the PluginRegistry only dispatches to plugins that subscribe to the exact WidgetId!
        // If JS wants all events, we should change PluginRegistry or return a wildcard.
        // For now, let's just subscribe to a few hardcoded ones or let PluginRegistry send all events if empty.
        // Actually, we can return the global ALL slice if we had one.
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
        // We will pass the event to JS `onEvent(event_json)`
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
                    UiEvent::Scroll(id_opt, dx, dy) => {
                        let id_str = match id_opt {
                            Some(id) => format!(r#""{}""#, id),
                            None => "null".to_string(),
                        };
                        format!(r#"{{"type":"Scroll","id":{},"dx":{},"dy":{}}}"#, id_str, dx, dy)
                    }
                };

                let res: Result<String, _> = on_event_fn.call((event_json,));
                if let Ok(_cmds_json) = res {
                    // Expect JS to return an array of DrawCommands or Actions in JSON
                    // e.g. [{"type": "DrawRect", "id": 123, "x": 0, "w": 50, ...}]
                    // For now, we can manually implement this logic in rust or JS.
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
}
