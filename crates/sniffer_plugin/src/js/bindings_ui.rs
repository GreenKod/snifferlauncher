use crate::js::permission_manager::permission_granted;
use crate::{dev_err, dev_log};
use obfstr::obfstr;
use rquickjs::{Ctx, Function, Object};
use sniffer_core::types::Element;
use std::sync::{Arc, Mutex};

pub enum UiMutation {
    SetUi(Element),
    UpdateStyle {
        id: String,
        property: String,
        value: String,
    },
    SetText {
        id: String,
        text: String,
    },
    InsertChild {
        parent_id: String,
        child: Element,
    },
    RemoveNode {
        id: String,
    },
}

#[derive(Default)]
pub struct MutationBuffer {
    pub mutations: Mutex<Vec<UiMutation>>,
}

impl MutationBuffer {
    pub fn flush(&self, ui_tree: &Arc<Mutex<Option<Element>>>) {
        let mut muts = match self.mutations.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        if muts.is_empty() {
            return;
        }

        if let Ok(mut tree_lock) = ui_tree.lock() {
            for m in muts.drain(..) {
                match m {
                    UiMutation::SetUi(new_elem) => {
                        *tree_lock = Some(new_elem);
                    }
                    UiMutation::UpdateStyle {
                        id,
                        property,
                        value,
                    } => {
                        if let Some(ref mut root) = *tree_lock {
                            root.mutate_style(&id, &property, &value);
                        }
                    }
                    UiMutation::SetText { id, text } => {
                        if let Some(ref mut root) = *tree_lock {
                            root.mutate_text(&id, &text);
                        }
                    }
                    UiMutation::InsertChild { parent_id, child } => {
                        if let Some(ref mut root) = *tree_lock {
                            root.insert_child(&parent_id, child);
                        }
                    }
                    UiMutation::RemoveNode { id } => {
                        if let Some(ref mut root) = *tree_lock {
                            root.remove_node(&id);
                        }
                    }
                }
            }
            sniffer_core::types::UI_VERSION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }
}

#[allow(clippy::too_many_lines)]
pub fn register_ui_bindings<'js>(
    ctx: &Ctx<'js>,
    globals: &Object<'js>,
    ui_tree: Arc<Mutex<Option<Element>>>,
    plugin_permissions: &[String],
    granted_permissions: Arc<Mutex<Vec<String>>>,
    cache_path: Option<std::path::PathBuf>,
) {
    let mutation_buffer = Arc::new(MutationBuffer::default());

    // host_set_ui
    let mut_set_ui = mutation_buffer.clone();
    let tree_set_ui = ui_tree.clone();
    let plugin_permissions_ui = plugin_permissions.to_vec();
    let granted_permissions_ui = granted_permissions.clone();
    let set_ui_func = Function::new(ctx.clone(), move |json_str: String| {
        if !permission_granted(
            obfstr!("plugin.permission.UI"),
            &plugin_permissions_ui,
            &granted_permissions_ui,
        ) {
            return;
        }

        match serde_json::from_str::<Element>(&json_str) {
            Ok(parsed) => {
                if let Ok(mut lock) = mut_set_ui.mutations.lock() {
                    lock.push(UiMutation::SetUi(parsed.clone()));
                }
                mut_set_ui.flush(&tree_set_ui);
                if let Some(path) = cache_path.clone()
                    && let Ok(bytes) = postcard::to_allocvec(&parsed)
                {
                    std::thread::spawn(move || {
                        let _ = std::fs::write(path, bytes);
                    });
                }
            }
            Err(e) => {
                let col = e.column();
                let start = col.saturating_sub(50);
                let end = (col + 50).min(json_str.len());
                let snippet = if start < json_str.len() {
                    &json_str[start..end]
                } else {
                    ""
                };
                dev_err!(
                    "{}: {e} (around col {col}: '{snippet}')",
                    obfstr!("JS Error: Failed to parse host_set_ui JSON")
                );
            }
        }
    })
    .unwrap();
    globals.set(obfstr!("host_set_ui"), set_ui_func).unwrap();

    // host_update_style
    let mut_update_style = mutation_buffer.clone();
    let tree_update_style = ui_tree.clone();
    let plugin_permissions_style = plugin_permissions.to_vec();
    let granted_permissions_style = granted_permissions.clone();
    let update_style_func = Function::new(
        ctx.clone(),
        move |id: String, property: String, value: String| {
            if !permission_granted(
                obfstr!("plugin.permission.UI"),
                &plugin_permissions_style,
                &granted_permissions_style,
            ) {
                return;
            }

            if let Ok(mut lock) = mut_update_style.mutations.lock() {
                lock.push(UiMutation::UpdateStyle {
                    id,
                    property,
                    value,
                });
            }
            mut_update_style.flush(&tree_update_style);
        },
    )
    .unwrap();
    globals
        .set(obfstr!("host_update_style"), update_style_func)
        .unwrap();

    // host_set_text
    let mut_set_text = mutation_buffer.clone();
    let tree_set_text = ui_tree.clone();
    let plugin_permissions_text = plugin_permissions.to_vec();
    let granted_permissions_text = granted_permissions.clone();
    let set_text_func = Function::new(ctx.clone(), move |id: String, text: String| {
        if !permission_granted(
            obfstr!("plugin.permission.UI"),
            &plugin_permissions_text,
            &granted_permissions_text,
        ) {
            return;
        }

        if let Ok(mut lock) = mut_set_text.mutations.lock() {
            lock.push(UiMutation::SetText { id, text });
        }
        mut_set_text.flush(&tree_set_text);
    })
    .unwrap();
    globals
        .set(obfstr!("host_set_text"), set_text_func)
        .unwrap();

    // host_insert_child
    let mut_insert_child = mutation_buffer.clone();
    let tree_insert_child = ui_tree.clone();
    let plugin_permissions_insert = plugin_permissions.to_vec();
    let granted_permissions_insert = granted_permissions.clone();
    let insert_child_func =
        Function::new(ctx.clone(), move |parent_id: String, child_json: String| {
            if !permission_granted(
                obfstr!("plugin.permission.UI"),
                &plugin_permissions_insert,
                &granted_permissions_insert,
            ) {
                return;
            }

            if let Ok(parsed_child) = serde_json::from_str::<Element>(&child_json) {
                if let Ok(mut lock) = mut_insert_child.mutations.lock() {
                    lock.push(UiMutation::InsertChild {
                        parent_id,
                        child: parsed_child,
                    });
                }
                mut_insert_child.flush(&tree_insert_child);
            } else {
                dev_err!(
                    "{}",
                    obfstr!("JS Error: Failed to parse child JSON in host_insert_child")
                );
            }
        })
        .unwrap();
    globals
        .set(obfstr!("host_insert_child"), insert_child_func)
        .unwrap();

    // host_remove_node
    let mut_remove_node = mutation_buffer;
    let tree_remove_node = ui_tree;
    let plugin_permissions_remove = plugin_permissions.to_vec();
    let granted_permissions_remove = granted_permissions;
    let remove_node_func = Function::new(ctx.clone(), move |id: String| {
        if !permission_granted(
            obfstr!("plugin.permission.UI"),
            &plugin_permissions_remove,
            &granted_permissions_remove,
        ) {
            return;
        }

        if let Ok(mut lock) = mut_remove_node.mutations.lock() {
            lock.push(UiMutation::RemoveNode { id });
        }
        mut_remove_node.flush(&tree_remove_node);
    })
    .unwrap();
    globals
        .set(obfstr!("host_remove_node"), remove_node_func)
        .unwrap();

    // host_get_binary_state
    fn get_binary_state<'js>(ctx: Ctx<'js>) -> rquickjs::Result<rquickjs::ArrayBuffer<'js>> {
        let state = sniffer_core::types::AppState {
            click_count: 42,
            screen_width: f32::from_bits(
                sniffer_core::types::SCREEN_WIDTH.load(std::sync::atomic::Ordering::Relaxed),
            ),
            screen_height: f32::from_bits(
                sniffer_core::types::SCREEN_HEIGHT.load(std::sync::atomic::Ordering::Relaxed),
            ),
        };
        let bytes = postcard::to_allocvec(&state).unwrap_or_default();
        rquickjs::ArrayBuffer::new(ctx, bytes)
    }
    let get_binary_state_func = Function::new(ctx.clone(), get_binary_state).unwrap();
    globals
        .set(obfstr!("host_get_binary_state"), get_binary_state_func)
        .unwrap();

    // host_send_binary_event
    let send_binary_event_func = Function::new(ctx.clone(), |buffer: rquickjs::ArrayBuffer<'_>| {
        if let Some(bytes) = buffer.as_bytes() {
            if let Ok(state) = postcard::from_bytes::<sniffer_core::types::AppState>(bytes) {
                dev_log!(
                    "{}: {state:?}",
                    obfstr!("JS sent binary state via ArrayBuffer")
                );
            } else {
                dev_err!("{}", obfstr!("Failed to deserialize binary event from JS."));
            }
        }
    })
    .unwrap();
    globals
        .set(obfstr!("host_send_binary_event"), send_binary_event_func)
        .unwrap();
}
