use crate::core::ui::data_map::{DataMap, DataValue, DrawCommand};
use crate::core::ui::event::UiEvent;
use crate::core::ui::style_map::StyleMap;
use crate::core::ui::widget::WidgetId;
use crate::plugin::r#trait::UiPlugin;
use std::sync::Mutex;
use wasmi::{Caller, Engine, Func, Linker, Module, Store};

pub struct HostState {
    pub memory: Option<wasmi::Memory>,
    pub styles: Option<StyleMap>,
    pub data: Option<DataMap>,
    pub actions: Option<std::sync::Arc<std::sync::Mutex<Vec<crate::core::types::Action>>>>,
}

/// A plugin that executes logic securely within a WebAssembly sandbox.
pub struct WasmPlugin {
    subscriptions: Vec<WidgetId>,
    store: Mutex<Store<HostState>>,
    instance: wasmi::Instance,
}

impl WasmPlugin {
    /// Instantiate a new Wasm plugin from the raw `.wasm` byte slice.
    ///
    /// # Errors
    /// Returns an error if the Wasm module fails to compile, instantiate, or if required exports are missing.
    #[allow(clippy::too_many_lines)]
    pub fn new(wasm_bytes: &[u8]) -> Result<Self, String> {
        let engine = Engine::default();
        let module = Module::new(&engine, wasm_bytes).map_err(|e| e.to_string())?;

        let state = HostState {
            memory: None,
            styles: None,
            data: None,
            actions: None,
        };
        let mut store = Store::new(&engine, state);
        let mut linker = Linker::new(&engine);

        let host_update_text = Func::wrap(
            &mut store,
            |caller: Caller<'_, HostState>, widget_id: i64, text_ptr: i32, text_len: i32| {
                let Some(memory) = caller.data().memory else {
                    return;
                };
                let Some(data) = &caller.data().data else {
                    return;
                };

                let Ok(len) = usize::try_from(text_len) else {
                    return;
                };
                let Ok(ptr) = usize::try_from(text_ptr) else {
                    return;
                };

                let mut text_buf = vec![0u8; len];
                if memory.read(&caller, ptr, &mut text_buf).is_ok()
                    && let Ok(text) = String::from_utf8(text_buf)
                {
                    #[allow(clippy::cast_sign_loss)]
                    data.set(widget_id as u64, "label", DataValue::Text(text));
                }
            },
        );
        linker
            .define("env", "host_update_text", host_update_text)
            .map_err(|e| e.to_string())?;

        let host_update_bg_color = Func::wrap(
            &mut store,
            |caller: Caller<'_, HostState>, widget_id: i64, color: u32| {
                let Some(styles) = &caller.data().styles else {
                    return;
                };
                #[allow(clippy::cast_sign_loss)]
                let w_id = widget_id as u64;
                styles.mutate(w_id, |s| s.background_color = Some(color));
            },
        );
        linker
            .define("env", "host_update_bg_color", host_update_bg_color)
            .map_err(|e| e.to_string())?;

        let host_draw_rect = Func::wrap(
            &mut store,
            |caller: Caller<'_, HostState>,
             widget_id: i64,
             x: f32,
             y: f32,
             w: f32,
             h: f32,
             color: u32,
             radius: f32| {
                let Some(data) = &caller.data().data else {
                    return;
                };
                #[allow(clippy::cast_sign_loss)]
                let w_id = widget_id as u64;
                let mut list = match data.get(w_id, "draw_list") {
                    Some(DataValue::DrawList(l)) => l,
                    _ => Vec::new(),
                };
                list.push(DrawCommand::Rect {
                    x,
                    y,
                    w,
                    h,
                    color,
                    radius,
                });
                data.set(w_id, "draw_list", DataValue::DrawList(list));
            },
        );
        linker
            .define("env", "host_draw_rect", host_draw_rect)
            .map_err(|e| e.to_string())?;

        let host_draw_circle = Func::wrap(
            &mut store,
            |caller: Caller<'_, HostState>,
             widget_id: i64,
             cx: f32,
             cy: f32,
             r: f32,
             color: u32| {
                let Some(data) = &caller.data().data else {
                    return;
                };
                #[allow(clippy::cast_sign_loss)]
                let w_id = widget_id as u64;
                let mut list = match data.get(w_id, "draw_list") {
                    Some(DataValue::DrawList(l)) => l,
                    _ => Vec::new(),
                };
                list.push(DrawCommand::Circle { cx, cy, r, color });
                data.set(w_id, "draw_list", DataValue::DrawList(list));
            },
        );
        linker
            .define("env", "host_draw_circle", host_draw_circle)
            .map_err(|e| e.to_string())?;

        let host_clear_draw_list = Func::wrap(
            &mut store,
            |caller: Caller<'_, HostState>, widget_id: i64| {
                let Some(data) = &caller.data().data else {
                    return;
                };
                #[allow(clippy::cast_sign_loss)]
                let w_id = widget_id as u64;
                data.clear(w_id, "draw_list");
            },
        );
        linker
            .define("env", "host_clear_draw_list", host_clear_draw_list)
            .map_err(|e| e.to_string())?;

        let host_clear_label = Func::wrap(
            &mut store,
            |caller: Caller<'_, HostState>, widget_id: i64| {
                let Some(data) = &caller.data().data else {
                    return;
                };
                #[allow(clippy::cast_sign_loss)]
                let w_id = widget_id as u64;
                data.clear(w_id, "label");
            },
        );
        linker
            .define("env", "host_clear_label", host_clear_label)
            .map_err(|e| e.to_string())?;

        let host_clear_style = Func::wrap(
            &mut store,
            |caller: Caller<'_, HostState>, widget_id: i64| {
                let Some(styles) = &caller.data().styles else {
                    return;
                };
                #[allow(clippy::cast_sign_loss)]
                let w_id = widget_id as u64;
                styles.clear(w_id);
            },
        );
        linker
            .define("env", "host_clear_style", host_clear_style)
            .map_err(|e| e.to_string())?;

        let host_set_visible = Func::wrap(
            &mut store,
            |caller: Caller<'_, HostState>, widget_id: i64, visible: i32| {
                let Some(data) = &caller.data().data else {
                    return;
                };
                #[allow(clippy::cast_sign_loss)]
                let w_id = widget_id as u64;
                data.set(w_id, "visible", DataValue::Visible(visible != 0));
            },
        );
        linker
            .define("env", "host_set_visible", host_set_visible)
            .map_err(|e| e.to_string())?;

        let host_execute_action = Func::wrap(
            &mut store,
            |caller: Caller<'_, HostState>, action_id: i32| {
                let Some(actions) = &caller.data().actions else {
                    return;
                };
                let action = match action_id {
                    1 => crate::core::types::Action::OpenSettings,
                    2 => crate::core::types::Action::OpenContacts,
                    3 => crate::core::types::Action::OpenCamera,
                    // If we had LaunchApp(String), we'd need host_execute_launch_app(ptr, len)
                    // For now, let's map generic actions for demonstration.
                    _ => return,
                };
                if let Ok(mut q) = actions.lock() {
                    q.push(action);
                }
            },
        );
        linker
            .define("env", "host_execute_action", host_execute_action)
            .map_err(|e| e.to_string())?;

        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| e.to_string())?
            .start(&mut store)
            .map_err(|e| e.to_string())?;

        if let Some(mem) = instance
            .get_export(&store, "memory")
            .and_then(wasmi::Extern::into_memory)
        {
            store.data_mut().memory = Some(mem);
        }

        Ok(Self {
            subscriptions: vec![
                crate::core::ui::widget::ids::BTN_SETTINGS,
                crate::core::ui::widget::ids::BTN_CONTACTS,
                crate::core::ui::widget::ids::BTN_CAMERA,
            ],
            store: Mutex::new(store),
            instance,
        })
    }
}

impl UiPlugin for WasmPlugin {
    fn subscriptions(&self) -> &[WidgetId] {
        &self.subscriptions
    }

    fn on_event(
        &self,
        event: &UiEvent,
        styles: &StyleMap,
        data: &DataMap,
        actions: &std::sync::Arc<std::sync::Mutex<Vec<crate::core::types::Action>>>,
    ) {
        let Ok(mut store) = self.store.lock() else {
            return;
        };

        store.data_mut().styles = Some(styles.clone());
        store.data_mut().data = Some(data.clone());
        store.data_mut().actions = Some(actions.clone());

        if let Some(func) = self
            .instance
            .get_export(&*store, "on_event")
            .and_then(wasmi::Extern::into_func)
        {
            #[allow(clippy::cast_possible_wrap)]
            let widget_id = event.widget_id() as i64;
            let (event_type, w, h, mx, my) = match event {
                UiEvent::Click(_, w, h) => (1, *w, *h, 0.0, 0.0),
                UiEvent::Hover(_, w, h, mx, my) => (2, *w, *h, *mx, *my),
                UiEvent::HoverEnd(_) => (3, 0.0, 0.0, 0.0, 0.0),
            };

            let _ = func
                .typed::<(i64, i32, f32, f32, f32, f32), ()>(&*store)
                .and_then(|f| f.call(&mut *store, (widget_id, event_type, w, h, mx, my)));
        }

        store.data_mut().styles = None;
        store.data_mut().data = None;
        store.data_mut().actions = None;
    }
}
