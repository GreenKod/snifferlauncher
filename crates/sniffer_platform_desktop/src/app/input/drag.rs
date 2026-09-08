use super::super::helpers::get_max_scroll_for_lay;
use super::super::state::AppState;
use sniffer_core::Point;
use sniffer_core::layout::LayoutNode;
use sniffer_core::types::Element;
use sniffer_core::ui::event::UiEvent;

/// Processes active drag events on the desktop window.
#[allow(clippy::cast_precision_loss)]
pub fn handle_drag_events(
    app: &mut AppState,
    drag_events: &[(f32, f32)],
    root_element: &Element,
    layout_tree: &LayoutNode,
    scaled_last_mouse_pos: Point,
    scale_x: f32,
    scale_y: f32,
) {
    for &(dx, dy) in drag_events {
        let s_dx = dx * scale_x;
        let s_dy = dy * scale_y;
        app.total_drag_dist += s_dx.abs() + s_dy.abs();
        app.last_drag_delta = (s_dx, s_dy);

        app.drag_history
            .push_back((s_dx, s_dy, std::time::Instant::now()));
        if app.drag_history.len() > 8 {
            app.drag_history.pop_front();
        }

        if app.active_scrollview_drag.is_none() {
            if let Some((
                Element::ScrollView {
                    id, capture_drag, ..
                },
                _,
            )) = sniffer_render::draw::find_hovered_scrollview(
                root_element,
                layout_tree,
                scaled_last_mouse_pos,
            ) {
                if capture_drag.unwrap_or(true) {
                    app.active_scrollview_drag = id
                        .as_deref()
                        .map(|id_str| sniffer_core::ui::widget::fnv1a(id_str.as_bytes()));
                }
            }
        }

        if let Some(sv_id) = app.active_scrollview_drag {
            if let Some(phys) = app.scroll_physics.get_mut(&sv_id) {
                if phys.snap_x.is_some() {
                    phys.apply_drag(s_dx);
                    phys.snap_target_x = None;
                } else {
                    phys.apply_drag_y(s_dy);
                }
                phys.is_dragging = true;
            } else {
                let target = sniffer_render::draw::find_hovered_scrollview(
                    root_element,
                    layout_tree,
                    scaled_last_mouse_pos,
                );
                let max_scroll = target.map_or((0.0, 0.0), |(_, lay)| get_max_scroll_for_lay(lay));
                app.event_bus.push(UiEvent::Scroll(
                    Some(sv_id),
                    s_dx,
                    s_dy,
                    max_scroll.0,
                    max_scroll.1,
                ));
            }
        }
    }
}
