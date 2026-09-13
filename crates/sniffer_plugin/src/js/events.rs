use sniffer_core::ui::event::UiEvent;

/// Serializes a [`UiEvent`] into a JSON string suitable for consumption by QuickJS `onEvent`.
#[must_use]
pub fn serialize_ui_event_to_json(event: &UiEvent, plugin_id: &str) -> String {
    match event {
        UiEvent::Click(id, w, h) => {
            crate::logger::info(
                plugin_id,
                &format!("Dispatching Click: id={id}, w={w}, h={h}"),
            );
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
            format!(
                r#"{{"type":"Scroll","id":{id_str},"dx":{dx},"dy":{dy},"max_x":{max_x},"max_y":{max_y}}}"#
            )
        }
        UiEvent::WindowResized(w, h) => {
            format!(r#"{{"type":"WindowResized","w":{w},"h":{h}}}"#)
        }
        UiEvent::PageSnapped { widget_id, page } => {
            format!(r#"{{"type":"PageSnapped","widget_id":{widget_id},"page":{page}}}"#)
        }
        UiEvent::SwipeUp => r#"{"type":"SwipeUp"}"#.to_string(),
        UiEvent::SwipeDown => r#"{"type":"SwipeDown"}"#.to_string(),
    }
}
