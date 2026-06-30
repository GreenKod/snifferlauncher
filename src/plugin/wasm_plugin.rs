use crate::core::ui::data_map::{DataMap, DataValue};
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
    pub fn new(wasm_bytes: &[u8]) -> Result<Self, String> {
        let engine = Engine::default();
        let module = Module::new(&engine, wasm_bytes).map_err(|e| e.to_string())?;

        let state = HostState {
            memory: None,
            styles: None,
            data: None,
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
                let mut style = styles.get(w_id).unwrap_or_default();
                style.background_color = Some(color);
                styles.set(w_id, style);
            },
        );
        linker
            .define("env", "host_update_bg_color", host_update_bg_color)
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

    fn on_event(&self, event: &UiEvent, styles: &StyleMap, data: &DataMap) {
        let Ok(mut store) = self.store.lock() else {
            return;
        };

        store.data_mut().styles = Some(styles.clone());
        store.data_mut().data = Some(data.clone());

        if let Some(func) = self
            .instance
            .get_export(&*store, "on_event")
            .and_then(wasmi::Extern::into_func)
        {
            #[allow(clippy::cast_possible_wrap)]
            let widget_id = event.widget_id() as i64;
            let event_type = match event {
                UiEvent::Click(_) => 1,
                UiEvent::Hover(_) => 2,
                UiEvent::HoverEnd(_) => 3,
            };

            let _ = func
                .typed::<(i64, i32), ()>(&*store)
                .and_then(|f| f.call(&mut *store, (widget_id, event_type)));
        }

        store.data_mut().styles = None;
        store.data_mut().data = None;
    }
}
