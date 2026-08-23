use super::super::helpers::resolve_active_scroll;
use super::super::state::AppState;
use sniffer_core::Point;
use sniffer_core::layout::LayoutNode;
use sniffer_core::types::Element;
use sniffer_core::ui::event::UiEvent;
use sniffer_render::draw::{find_clicked_button_with_scroll, find_hovered_button_with_scroll};

/// Processes mouse hover and pointer position tracking.
pub fn handle_mouse_move(
    app: &mut AppState,
    root_element: &Element,
    layout_tree: &LayoutNode,
    scaled_last_mouse_pos: Point,
) {
    let prev_hovered = app.hovered_btn;
    let hovered_data = find_hovered_button_with_scroll(
        root_element,
        layout_tree,
        scaled_last_mouse_pos,
        &|id_opt, sx, sy| resolve_active_scroll(&app.scroll_physics, id_opt, sx, sy),
    );
    app.hovered_btn = hovered_data.map(|(id, _)| id);

    match (prev_hovered, hovered_data) {
        (Some(prev), Some((current, rect))) if prev != current => {
            app.event_bus.push(UiEvent::HoverEnd(prev));
            app.event_bus.push(UiEvent::Hover(
                current,
                rect.width,
                rect.height,
                scaled_last_mouse_pos.x - rect.x,
                scaled_last_mouse_pos.y - rect.y,
            ));
        }
        (Some(prev), Some((current, rect))) if prev == current => {
            app.event_bus.push(UiEvent::Hover(
                current,
                rect.width,
                rect.height,
                scaled_last_mouse_pos.x - rect.x,
                scaled_last_mouse_pos.y - rect.y,
            ));
        }
        (None, Some((current, rect))) => {
            app.event_bus.push(UiEvent::Hover(
                current,
                rect.width,
                rect.height,
                scaled_last_mouse_pos.x - rect.x,
                scaled_last_mouse_pos.y - rect.y,
            ));
        }
        (Some(prev), None) => {
            app.event_bus.push(UiEvent::HoverEnd(prev));
        }
        _ => {}
    }
}

/// Processes mouse button press / pointer down events.
pub fn handle_mouse_down(
    app: &mut AppState,
    root_element: &Element,
    layout_tree: &LayoutNode,
    clicked_pt: Point,
) {
    app.mouse_down_pos = Some(clicked_pt);
    app.total_drag_dist = 0.0;
    app.drag_history.clear();

    if let Some((
        Element::ScrollView {
            id, capture_drag, ..
        },
        _,
    )) = sniffer_render::draw::find_hovered_scrollview(root_element, layout_tree, clicked_pt)
    {
        if capture_drag.unwrap_or(true) {
            app.active_scrollview_drag = id
                .as_deref()
                .map(|id_str| sniffer_core::ui::widget::fnv1a(id_str.as_bytes()));
        }
        app.kinetic_scrolls.clear();
    }

    if let Some((clicked_btn, _)) =
        find_clicked_button_with_scroll(root_element, layout_tree, clicked_pt, &|id_opt, sx, sy| {
            resolve_active_scroll(&app.scroll_physics, id_opt, sx, sy)
        })
    {
        app.event_bus.push(UiEvent::PointerDown(clicked_btn));
    } else {
        app.event_bus.push(UiEvent::PointerDown(0));
    }
}
