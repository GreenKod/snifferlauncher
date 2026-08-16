use crate::core::ui::data_map::DataMap;
use crate::core::ui::event::UiEvent;
use crate::core::ui::style_map::StyleMap;
use crate::core::WidgetId;
use crate::core::types::Element;
use crate::dev_err;
use crate::plugin::js::engine::create_engine;
use crate::plugin::js::register_host_api;
use crate::plugin::js::HostApiConfig;
use crate::plugin::registry::{ApiMap, BroadcastQueue};
use crate::plugin::UiPlugin;
use crossbeam_channel::{Sender, unbounded};
use rquickjs::Value;
use std::sync::{Arc, Mutex};
use std::thread;

pub const IPC_PREAMBLE: &str = include_str!("ipc_preamble.js");

pub enum PluginMsg {
    Event(UiEvent),
    Tick,
    Broadcast {
        channel: String,
        payload: String,
    },
    ApiCall {
        name: String,
        payload: String,
        responder: Sender<Option<String>>,
    },
    Suspend,
    Resume,
    Unload,
}

#[allow(dead_code)]
pub struct JsPlugin {
    pub(crate) msg_tx: Sender<PluginMsg>,
    pub(crate) ui_tree: Arc<Mutex<Option<Element>>>,
    pub(crate) subscriptions: Vec<WidgetId>,
}

#[allow(clippy::non_send_fields_in_send_ty)]
unsafe impl Send for JsPlugin {}
unsafe impl Sync for JsPlugin {}

pub struct JsPluginConfig {
    pub script_content: String,
    pub plugin_id: String,
    pub vault: Arc<crate::core::vault::DataVault>,
    pub action_queue: Arc<Mutex<Vec<crate::core::types::Action>>>,
    pub api_map: ApiMap,
    pub broadcast_queue: BroadcastQueue,
    pub permissions: Vec<String>,
    pub default_settings: serde_json::Value,
    pub cached_ui: Option<Element>,
    pub cache_path: Option<std::path::PathBuf>,
}

impl JsPlugin {
    pub fn new(config: JsPluginConfig) -> Result<Self, String> {
        let (msg_tx, msg_rx) = unbounded::<PluginMsg>();
        let ui_tree = Arc::new(Mutex::new(config.cached_ui));
        let ui_tree_worker = ui_tree.clone();

        #[cfg(not(target_os = "android"))]
        let initial_granted = config.permissions.clone();

        #[cfg(target_os = "android")]
        let initial_granted: Vec<String> = config
            .permissions
            .iter()
            .filter(|p| p.starts_with("plugin.permission.") || p.starts_with("android.permission."))
            .cloned()
            .collect();

        let granted_permissions = Arc::new(Mutex::new(initial_granted));
        let permissions_clone = config.permissions.clone();

        let script_content = config.script_content;
        let plugin_id = config.plugin_id;
        let vault = config.vault;
        let action_queue = config.action_queue;
        let api_map = config.api_map;
        let broadcast_queue = config.broadcast_queue;
        let default_settings = config.default_settings;
        let cache_path = config.cache_path;
        let msg_tx_worker = msg_tx.clone();

        thread::spawn(move || {
            let (runtime, context) = match create_engine() {
                Ok(res) => res,
                Err(e) => {
                    dev_err!("QuickJS worker engine error: {e}");
                    return;
                }
            };

            let init_res = context.with(|ctx| {
                register_host_api(
                    &ctx,
                    HostApiConfig {
                        plugin_id: plugin_id.clone(),
                        msg_tx: msg_tx_worker,
                        vault,
                        ui_tree: ui_tree_worker,
                        action_queue,
                        api_map,
                        broadcast_queue,
                        plugin_permissions: permissions_clone,
                        granted_permissions,
                        default_settings,
                        cache_path,
                    },
                );

                if let Err(e) = ctx.eval::<Value, _>(IPC_PREAMBLE.as_bytes()) {
                    if let Some(exc) = ctx.catch().as_exception() {
                        let msg = exc.message().unwrap_or_default();
                        let stack = exc.stack().unwrap_or_default();
                        return Err(format!("IPC_PREAMBLE error: {msg}\n{stack}"));
                    }
                    return Err(format!("IPC_PREAMBLE error: {e}"));
                }

                if let Err(e) = ctx.eval::<Value, _>(script_content.as_bytes()) {
                    if let Some(exc) = ctx.catch().as_exception() {
                        let msg = exc.message().unwrap_or_default();
                        let stack = exc.stack().unwrap_or_default();
                        dev_err!("SCRIPT FAIL {plugin_id}: {msg} | {stack}");
                        let _ = std::fs::write(format!("/data/user/0/com.greenkod.snifferlauncher/{plugin_id}.js"), &script_content);
                        return Err(format!("Script eval error in plugin: {msg}\n{stack}"));
                    }
                    return Err(format!("Script eval error in plugin: {e}"));
                }

                Ok::<(), String>(())
            });

            if let Err(e) = init_res {
                dev_err!("QuickJS init worker error: {e}");
                return;
            }

            let mut gc_count = 0u32;
            let mut is_suspended = false;
            while let Ok(msg) = msg_rx.recv() {
                match msg {
                    PluginMsg::Event(event) => {
                        context.with(|ctx| {
                            let globals = ctx.globals();
                            if let Ok(on_event_fn) = globals.get::<_, rquickjs::Function>("onEvent") {
                                let event_json = match &event {
                                    UiEvent::Click(id, w, h) => {
                                        crate::dev_log!("[JS Worker] Dispatching Click to JS: id={id}, w={w}, h={h}");
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
                                    UiEvent::Scroll(id_opt, dx, dy, max_x, max_y) => {
                                        let id_str = match id_opt {
                                            Some(id) => format!(r#""{id}""#),
                                            None => "null".to_string(),
                                        };
                                        format!(r#"{{"type":"Scroll","id":{id_str},"dx":{dx},"dy":{dy},"max_x":{max_x},"max_y":{max_y}}}"#)
                                    }
                                    UiEvent::WindowResized(w, h) => {
                                        format!(r#"{{"type":"WindowResized","w":{w},"h":{h}}}"#)
                                    }
                                    UiEvent::PageSnapped { widget_id, page } => {
                                        format!(r#"{{"type":"PageSnapped","widget_id":{widget_id},"page":{page}}}"#)
                                    }
                                };
                                let res: Result<rquickjs::Value, _> = on_event_fn.call((event_json,));
                                if let Err(e) = res {
                                    let caught = ctx.catch();
                                    let exc = caught.as_exception();
                                    let msg = exc.as_ref().and_then(|x| x.message()).unwrap_or_default();
                                    let stack = exc.as_ref().and_then(|x| x.stack()).unwrap_or_default();
                                    crate::dev_err!("[JS Worker] onEvent failed in plugin '{}': {msg}\n{stack}\n{e}", plugin_id);
                                }
                            }
                        });
                    }
                    PluginMsg::Tick => {
                        if !is_suspended {
                            gc_count += 1;
                            if gc_count >= 300 {
                                gc_count = 0;
                                runtime.run_gc();
                            }
                            context.with(|ctx| {
                                if let Ok(handler) = ctx.globals().get::<_, rquickjs::Function>("_onTimerTick") {
                                    let _ = handler.call::<_, ()>(());
                                }
                            });
                        }
                    }
                    PluginMsg::Broadcast { channel, payload } => {
                        context.with(|ctx| {
                            if let Ok(handler) = ctx.globals().get::<_, rquickjs::Function>("_dispatchBroadcast") {
                                let _ = handler.call::<_, ()>((channel, payload));
                            }
                        });
                    }
                    PluginMsg::ApiCall {
                        name,
                        payload,
                        responder,
                    } => {
                        let mut result: Option<String> = None;
                        context.with(|ctx| {
                            if let Ok(handler) =
                                ctx.globals().get::<_, rquickjs::Function>(obfstr::obfstr!("_handleApiCall"))
                                && let Ok(ret) = handler.call::<_, rquickjs::Value>((name, payload))
                                && ret.is_string()
                            {
                                result = ret.as_string().and_then(|s| s.to_string().ok());
                            }
                        });
                        let _ = responder.send(result);
                    }
                    PluginMsg::Suspend => {
                        is_suspended = true;
                        runtime.run_gc();
                    }
                    PluginMsg::Resume => {
                        is_suspended = false;
                    }
                    PluginMsg::Unload => {
                        runtime.run_gc();
                        break;
                    }
                }
            }
        });

        Ok(Self {
            msg_tx,
            ui_tree,
            subscriptions: vec![],
        })
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
        let _ = self.msg_tx.send(PluginMsg::Event(event.clone()));
    }

    fn on_tick(&self) {
        let _ = self.msg_tx.send(PluginMsg::Tick);
    }

    fn on_broadcast(&self, channel: &str, payload_json: &str) {
        let _ = self.msg_tx.send(PluginMsg::Broadcast {
            channel: channel.to_string(),
            payload: payload_json.to_string(),
        });
    }

    fn on_suspend(&self) {
        let _ = self.msg_tx.send(PluginMsg::Suspend);
    }

    fn on_resume(&self) {
        let _ = self.msg_tx.send(PluginMsg::Resume);
    }

    fn on_unload(&self) {
        let _ = self.msg_tx.send(PluginMsg::Unload);
    }
}
