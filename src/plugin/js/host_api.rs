use crate::core::types::Element;
use crate::plugin::registry::{ApiEntry, ApiMap, BroadcastQueue};
use rquickjs::Function;
use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------
// SafeContext
//
// rquickjs::Context is !Send by default. We wrap it in a newtype and
// assert Send + Sync ourselves. This is safe because:
//   - The render loop is single-threaded; no two plugins ever run concurrently.
//   - The Mutex in ApiMap ensures exclusive access when cross-plugin calls occur.
// ---------------------------------------------------------------------------

struct SafeContext(rquickjs::Context);

#[allow(clippy::non_send_fields_in_send_ty)]
unsafe impl Send for SafeContext {}
unsafe impl Sync for SafeContext {}

// ---------------------------------------------------------------------------
// register_host_api
// ---------------------------------------------------------------------------

/// Configuration bundle passed to `register_host_api`.
/// Groups all shared state to stay within Clippy's argument-count limit.
pub struct HostApiConfig {
    pub plugin_id: String,
    pub context: rquickjs::Context,
    pub ui_tree: Arc<Mutex<Option<Element>>>,
    pub action_queue: Arc<Mutex<Vec<crate::core::types::Action>>>,
    pub api_map: ApiMap,
    pub broadcast_queue: BroadcastQueue,
    pub cache_path: Option<std::path::PathBuf>,
}

/// Inject all host-provided global functions into a QuickJS context.
///
/// # Parameters
/// - `cfg` — Configuration bundle containing shared state and context.
pub fn register_host_api(ctx: &rquickjs::Ctx, cfg: HostApiConfig) {
    let HostApiConfig {
        plugin_id,
        context,
        ui_tree,
        action_queue,
        api_map,
        broadcast_queue,
        cache_path,
    } = cfg;

    let globals = ctx.globals();

    // ------------------------------------------------------------------
    // host_set_ui
    // ------------------------------------------------------------------
    let tree_set_ui = ui_tree.clone();
    let set_ui_func = Function::new(
        ctx.clone(),
        move |json_str: String| match serde_json::from_str::<Element>(&json_str) {
            Ok(parsed) => {
                if let Ok(mut lock) = tree_set_ui.lock() {
                    *lock = Some(parsed.clone());
                    println!("host_set_ui successfully parsed and locked UI.");
                }
                if let Some(path) = &cache_path
                    && let Ok(bytes) = postcard::to_allocvec(&parsed)
                {
                    match std::fs::write(path, bytes) {
                        Ok(_) => {
                            println!("host_set_ui wrote to cache.");
                        }
                        Err(e) => {
                            println!("host_set_ui cache write failed: {e}");
                        }
                    }
                }
            }
            Err(e) => {
                println!("JS Error: Failed to parse host_set_ui JSON: {e}");
            }
        },
    )
    .unwrap();
    globals.set("host_set_ui", set_ui_func).unwrap();

    // ------------------------------------------------------------------
    // host_update_style
    // ------------------------------------------------------------------
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

    // ------------------------------------------------------------------
    // host_set_text
    // ------------------------------------------------------------------
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

    // ------------------------------------------------------------------
    // host_insert_child
    // ------------------------------------------------------------------
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

    // ------------------------------------------------------------------
    // host_remove_node
    // ------------------------------------------------------------------
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

    // ------------------------------------------------------------------
    // host_get_binary_state (Phase 2)
    // ------------------------------------------------------------------
    fn get_binary_state<'js>(
        ctx: rquickjs::Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::ArrayBuffer<'js>> {
        let state = crate::core::types::AppState {
            click_count: 42, // Dummy count for example
            screen_width: f32::from_bits(
                crate::core::types::SCREEN_WIDTH.load(std::sync::atomic::Ordering::Relaxed),
            ),
            screen_height: f32::from_bits(
                crate::core::types::SCREEN_HEIGHT.load(std::sync::atomic::Ordering::Relaxed),
            ),
        };
        let bytes = postcard::to_allocvec(&state).unwrap_or_default();
        rquickjs::ArrayBuffer::new(ctx, bytes)
    }
    let get_binary_state_func = Function::new(ctx.clone(), get_binary_state).unwrap();
    globals
        .set("host_get_binary_state", get_binary_state_func)
        .unwrap();

    // ------------------------------------------------------------------
    // host_send_binary_event (Phase 2)
    // ------------------------------------------------------------------
    let send_binary_event_func = Function::new(ctx.clone(), |buffer: rquickjs::ArrayBuffer<'_>| {
        if let Some(bytes) = buffer.as_bytes() {
            if let Ok(state) = postcard::from_bytes::<crate::core::types::AppState>(bytes) {
                println!("JS sent binary state via ArrayBuffer: {state:?}");
            } else {
                println!("Failed to deserialize binary event from JS.");
            }
        }
    })
    .unwrap();
    globals
        .set("host_send_binary_event", send_binary_event_func)
        .unwrap();

    // ------------------------------------------------------------------
    // host_log
    // ------------------------------------------------------------------
    let log_func = Function::new(ctx.clone(), |msg: String| {
        println!("JS Log: {msg}");
    })
    .unwrap();
    globals.set("host_log", log_func).unwrap();

    // ------------------------------------------------------------------
    // host_hash
    // ------------------------------------------------------------------
    let hash_func = Function::new(ctx.clone(), |s: String| -> String {
        crate::core::ui::widget::fnv1a(s.as_bytes()).to_string()
    })
    .unwrap();
    globals.set("host_hash", hash_func).unwrap();

    // ------------------------------------------------------------------
    // host_screen_width
    // ------------------------------------------------------------------
    let get_width_func = Function::new(ctx.clone(), || -> f32 {
        f32::from_bits(crate::core::types::SCREEN_WIDTH.load(std::sync::atomic::Ordering::Relaxed))
    })
    .unwrap();
    globals.set("host_screen_width", get_width_func).unwrap();

    // ------------------------------------------------------------------
    // host_screen_height
    // ------------------------------------------------------------------
    let get_height_func = Function::new(ctx.clone(), || -> f32 {
        f32::from_bits(crate::core::types::SCREEN_HEIGHT.load(std::sync::atomic::Ordering::Relaxed))
    })
    .unwrap();
    globals.set("host_screen_height", get_height_func).unwrap();

    // ------------------------------------------------------------------
    // host_create_image
    // ------------------------------------------------------------------
    let aq_img = action_queue.clone();
    let create_image_func = Function::new(ctx.clone(), move |id: String, src: String| {
        if let Ok(mut q) = aq_img.lock() {
            q.push(crate::core::types::Action::LoadImage { id, src });
        }
    })
    .unwrap();
    globals.set("host_create_image", create_image_func).unwrap();

    // ------------------------------------------------------------------
    // host_focus_input
    // ------------------------------------------------------------------
    let aq_focus = action_queue.clone();
    let focus_input_func = Function::new(ctx.clone(), move |id: String| {
        if let Ok(mut q) = aq_focus.lock() {
            q.push(crate::core::types::Action::FocusTextInput(id));
        }
    })
    .unwrap();
    globals.set("host_focus_input", focus_input_func).unwrap();

    // ------------------------------------------------------------------
    // host_blur_input
    // ------------------------------------------------------------------
    let aq_blur = action_queue.clone();
    let blur_input_func = Function::new(ctx.clone(), move || {
        if let Ok(mut q) = aq_blur.lock() {
            q.push(crate::core::types::Action::BlurTextInput);
        }
    })
    .unwrap();
    globals.set("host_blur_input", blur_input_func).unwrap();

    // ==================================================================
    // Inter-Plugin API host functions
    // ==================================================================

    // ------------------------------------------------------------------
    // host_register_api(name: String)
    //
    // Called by a plugin's `registerApi(name, handler)` JS shim.
    // Registers a Rust closure in the ApiMap. When another plugin calls
    // this API, the closure re-enters THIS plugin's QuickJS context and
    // invokes `globalThis._handleApiCall(name, payload_json)`.
    // ------------------------------------------------------------------
    let safe_ctx = SafeContext(context);
    let safe_ctx = Arc::new(safe_ctx);
    let api_map_register = api_map.clone();
    let owning_plugin_id = plugin_id.clone();

    let register_api_func = Function::new(ctx.clone(), move |name: String| {
        let ctx_clone = Arc::clone(&safe_ctx);
        let api_name_clone = name.clone();
        let owning_id = owning_plugin_id.clone();

        let callback: Arc<dyn Fn(String) -> Option<String> + Send + Sync> =
            Arc::new(move |payload_json: String| {
                let mut result: Option<String> = None;
                ctx_clone.0.with(|ctx| {
                    if let Ok(handler) =
                        ctx.globals().get::<_, rquickjs::Function>("_handleApiCall")
                        && let Ok(ret) = handler
                            .call::<_, rquickjs::Value>((api_name_clone.clone(), payload_json))
                        && ret.is_string()
                    {
                        result = ret.as_string().and_then(|s| s.to_string().ok());
                    }
                });
                result
            });

        if let Ok(mut map) = api_map_register.lock() {
            if map.contains_key(&name) {
                println!(
                    "[IPC] Warning: API '{name}' is already registered. Overwriting with plugin '{owning_id}'."
                );
            }
            map.insert(
                name.clone(),
                ApiEntry {
                    plugin_id: owning_id.clone(),
                    callback,
                },
            );
            println!("[IPC] Plugin '{owning_id}' registered API '{name}'.");
        }
    })
    .unwrap();
    globals.set("host_register_api", register_api_func).unwrap();

    // ------------------------------------------------------------------
    // host_call_api(name: String, payload_json: String) -> Option<String>
    //
    // Called by a plugin's `callApi(name, payload)` JS shim.
    // Looks up the API in the ApiMap and invokes the registered closure.
    // Returns the JSON-string result, or JS `null` if the API is absent.
    // ------------------------------------------------------------------
    let api_map_call = api_map.clone();
    let calling_plugin_id = plugin_id.clone();

    let call_api_func = Function::new(
        ctx.clone(),
        move |name: String, payload_json: String| -> Option<String> {
            let map = match api_map_call.lock() {
                Ok(m) => m,
                Err(_) => {
                    eprintln!("[IPC] Failed to lock ApiMap for call to '{name}'.");
                    return None;
                }
            };

            if let Some(entry) = map.get(&name) {
                println!(
                    "[IPC] Plugin '{calling_plugin_id}' calling API '{name}' (owned by '{}').",
                    entry.plugin_id
                );
                // Release the lock before invoking the callback to avoid deadlock
                // when the callee itself tries to lock the map.
                let callback = Arc::clone(&entry.callback);
                drop(map);
                callback(payload_json)
            } else {
                eprintln!("[IPC] Plugin '{calling_plugin_id}' tried to call unknown API '{name}'.");
                None
            }
        },
    )
    .unwrap();
    globals.set("host_call_api", call_api_func).unwrap();

    // ------------------------------------------------------------------
    // host_broadcast(channel: String, payload_json: String)
    //
    // Called by a plugin's `broadcastEvent(channel, data)` JS shim.
    // Pushes the event into the broadcast queue; it is drained and
    // dispatched to all plugins by PluginRegistry::dispatch() each frame.
    // ------------------------------------------------------------------
    let bq = broadcast_queue;
    let broadcast_plugin_id = plugin_id;
    let broadcast_func =
        Function::new(ctx.clone(), move |channel: String, payload_json: String| {
            println!("[IPC] Plugin '{broadcast_plugin_id}' broadcasting on channel '{channel}'.");
            if let Ok(mut q) = bq.lock() {
                q.push((channel, payload_json));
            }
        })
        .unwrap();
    globals.set("host_broadcast", broadcast_func).unwrap();
}
