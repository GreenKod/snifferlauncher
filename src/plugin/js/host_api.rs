use crate::core::types::Element;
use rquickjs::Function;
use std::sync::{Arc, Mutex};

pub fn register_host_api(
    ctx: &rquickjs::Ctx,
    ui_tree: Arc<Mutex<Option<Element>>>,
    action_queue: Arc<Mutex<Vec<crate::core::types::Action>>>,
) {
    let globals = ctx.globals();

    // host_set_ui
    let tree_set_ui = ui_tree.clone();
    let set_ui_func = Function::new(
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

    // host_send_binary_event (Phase 2)
    let send_binary_event_func = Function::new(ctx.clone(), |buffer: rquickjs::ArrayBuffer<'_>| {
        if let Some(bytes) = buffer.as_bytes() {
            if let Ok(state) = postcard::from_bytes::<crate::core::types::AppState>(bytes) {
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
        f32::from_bits(crate::core::types::SCREEN_WIDTH.load(std::sync::atomic::Ordering::Relaxed))
    })
    .unwrap();
    globals.set("host_screen_width", get_width_func).unwrap();

    // host_screen_height
    let get_height_func = Function::new(ctx.clone(), || -> f32 {
        f32::from_bits(crate::core::types::SCREEN_HEIGHT.load(std::sync::atomic::Ordering::Relaxed))
    })
    .unwrap();
    globals.set("host_screen_height", get_height_func).unwrap();

    // host_create_image
    let aq_img = action_queue.clone();
    let create_image_func = Function::new(ctx.clone(), move |id: String, src: String| {
        if let Ok(mut q) = aq_img.lock() {
            q.push(crate::core::types::Action::LoadImage { id, src });
        }
    })
    .unwrap();
    globals.set("host_create_image", create_image_func).unwrap();

    // host_focus_input
    let aq_focus = action_queue.clone();
    let focus_input_func = Function::new(ctx.clone(), move |id: String| {
        if let Ok(mut q) = aq_focus.lock() {
            q.push(crate::core::types::Action::FocusTextInput(id));
        }
    })
    .unwrap();
    globals.set("host_focus_input", focus_input_func).unwrap();

    // host_blur_input
    let aq_blur = action_queue.clone();
    let blur_input_func = Function::new(ctx.clone(), move || {
        if let Ok(mut q) = aq_blur.lock() {
            q.push(crate::core::types::Action::BlurTextInput);
        }
    })
    .unwrap();
    globals.set("host_blur_input", blur_input_func).unwrap();
}
