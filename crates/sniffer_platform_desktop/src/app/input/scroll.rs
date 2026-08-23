use super::super::helpers::{find_first_scrollview, get_max_scroll_for_lay};
use super::super::state::AppState;
use sniffer_core::Point;
use sniffer_core::layout::LayoutNode;
use sniffer_core::types::Element;
use sniffer_core::ui::event::UiEvent;

/// Processes mouse wheel scroll events and updates active scroll physics / kinetic velocity.
pub fn handle_scroll_events(
    app: &mut AppState,
    scroll_events: &[(f32, f32)],
    root_element: &Element,
    layout_tree: &LayoutNode,
    scaled_last_mouse_pos: Point,
) {
    for &(x, y) in scroll_events {
        let target = sniffer_render::draw::find_hovered_scrollview(
            root_element,
            layout_tree,
            scaled_last_mouse_pos,
        )
        .or_else(|| find_first_scrollview(root_element, layout_tree));

        if let Some((
            Element::ScrollView {
                id,
                scroll_sensitivity,
                dynamic_sensitivity,
                ..
            },
            lay,
        )) = target
        {
            let mut factor = scroll_sensitivity.unwrap_or(1.0);
            if dynamic_sensitivity.unwrap_or(false) && !lay.children.is_empty() {
                let view_height = lay.rect.height;
                let mut min_y = f32::MAX;
                let mut max_y = f32::MIN;
                for child in &lay.children {
                    if child.rect.y < min_y {
                        min_y = child.rect.y;
                    }
                    if child.rect.y + child.rect.height > max_y {
                        max_y = child.rect.y + child.rect.height;
                    }
                }
                let content_height = max_y - min_y;
                if view_height > 0.0 && content_height > view_height {
                    factor *= content_height / view_height;
                }
            }

            let sv_id = id
                .as_deref()
                .map(|id_str| sniffer_core::ui::widget::fnv1a(id_str.as_bytes()));

            if let Some(wid) = sv_id {
                if let Some(phys) = app.scroll_physics.get_mut(&wid) {
                    let delta = -x * 30.0 - y * 30.0;
                    phys.apply_drag(delta);
                    phys.release_drag(delta * 4.0);
                } else {
                    let max_scroll = get_max_scroll_for_lay(lay);
                    app.event_bus.push(UiEvent::Scroll(
                        Some(wid),
                        -x * 20.0 * factor,
                        -y * 20.0 * factor,
                        max_scroll.0,
                        max_scroll.1,
                    ));
                }
            }
        }
    }
}

/// Ticks active kinetic scrolls across frames.
pub fn tick_kinetic_scrolls(app: &mut AppState, root_element: &Element, layout_tree: &LayoutNode) {
    app.kinetic_scrolls.retain_mut(|k| {
        if k.velocity_x.abs() > 0.1 || k.velocity_y.abs() > 0.1 {
            let max_scroll = find_first_scrollview(root_element, layout_tree)
                .map_or((0.0, 0.0), |(_, lay)| get_max_scroll_for_lay(lay));
            app.event_bus.push(UiEvent::Scroll(
                Some(k.sv_id),
                k.velocity_x,
                k.velocity_y,
                max_scroll.0,
                max_scroll.1,
            ));
            k.velocity_x *= 0.92;
            k.velocity_y *= 0.92;
            true
        } else {
            false
        }
    });
}
