#![allow(clippy::pedantic, clippy::nursery)]

pub mod engine;
pub mod host_api;

use crate::core::types::Element;
use crate::core::ui::data_map::DataMap;
use crate::core::ui::event::UiEvent;
use crate::core::ui::style_map::StyleMap;
use crate::core::ui::widget::WidgetId;
use crate::plugin::r#trait::UiPlugin;
use rquickjs::{Context, Runtime, Value};
use std::sync::{Arc, Mutex};

use engine::create_engine;
use host_api::register_host_api;

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
        let (runtime, context) = create_engine()?;
        let ui_tree = Arc::new(Mutex::new(None));

        let plugin = Self {
            script_content,
            runtime,
            context,
            subscriptions: vec![],
            ui_tree: ui_tree.clone(),
            gc_counter: std::sync::Mutex::new(0),
            action_queue: action_queue.clone(),
        };

        plugin.init_js_env(ui_tree, action_queue)?;

        Ok(plugin)
    }

    fn init_js_env(
        &self,
        ui_tree: Arc<Mutex<Option<Element>>>,
        action_queue: Arc<Mutex<Vec<crate::core::types::Action>>>,
    ) -> Result<(), String> {
        self.context.with(|ctx| {
            register_host_api(&ctx, ui_tree, action_queue);

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
                            Some(id) => format!(r#""{}""#, id),
                            None => "null".to_string(),
                        };
                        format!(r#"{{"type":"PointerUp","id":{}}}"#, id_str)
                    }
                    UiEvent::Scroll(id_opt, dx, dy) => {
                        let id_str = match id_opt {
                            Some(id) => format!(r#""{}""#, id),
                            None => "null".to_string(),
                        };
                        format!(
                            r#"{{"type":"Scroll","id":{},"dx":{},"dy":{}}}"#,
                            id_str, dx, dy
                        )
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
}
