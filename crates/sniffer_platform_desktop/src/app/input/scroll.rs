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
        let eff_y = if y.abs() > 0.001 { y } else { -x };

        let target = sniffer_render::draw::find_hovered_scrollview(
            root_element,
            layout_tree,
            scaled_last_mouse_pos,
        )
        .or_else(|| find_first_scrollview(root_element, layout_tree));

        if app.is_shift_down {
            // Shift + Scroll: Strictly VERTICAL (Floor transitions or vertical list scrolling)
            if let Some((Element::ScrollView { id, .. }, _lay)) = target {
                let sv_id = id
                    .as_deref()
                    .map(|id_str| sniffer_core::ui::widget::fnv1a(id_str.as_bytes()));

                if let Some(wid) = sv_id {
                    if let Some(phys) = app.scroll_physics.get_mut(&wid) {
                        if phys.snap_x.is_some() {
                            // Floor 1 (Home Screen pager): Shift + wheel navigates floors
                            if eff_y < -0.1 {
                                app.event_bus.push(UiEvent::SwipeUp);
                            } else if eff_y > 0.1 {
                                app.event_bus.push(UiEvent::SwipeDown);
                            }
                        } else {
                            // Floor 2 (App Drawer vertical list)
                            if eff_y > 0.1 && phys.pos_y <= 0.01 {
                                // At the top of App Drawer and scrolling UP -> return to Floor 1
                                app.event_bus.push(UiEvent::SwipeDown);
                            } else {
                                // Scroll the vertical app list
                                let delta_y = -eff_y * 24.0;
                                phys.apply_drag_y(delta_y);
                                phys.release_drag_y(delta_y * 1.8);
                            }
                        }
                    } else if eff_y < -0.1 {
                        app.event_bus.push(UiEvent::SwipeUp);
                    } else if eff_y > 0.1 {
                        app.event_bus.push(UiEvent::SwipeDown);
                    }
                } else if eff_y < -0.1 {
                    app.event_bus.push(UiEvent::SwipeUp);
                } else if eff_y > 0.1 {
                    app.event_bus.push(UiEvent::SwipeDown);
                }
            } else if eff_y < -0.1 {
                app.event_bus.push(UiEvent::SwipeUp);
            } else if eff_y > 0.1 {
                app.event_bus.push(UiEvent::SwipeDown);
            }
        } else {
            // Normal scroll (without Shift): Strictly HORIZONTAL!
            if let Some((Element::ScrollView { id, .. }, _lay)) = target {
                let sv_id = id
                    .as_deref()
                    .map(|id_str| sniffer_core::ui::widget::fnv1a(id_str.as_bytes()));

                if let Some(wid) = sv_id {
                    if let Some(phys) = app.scroll_physics.get_mut(&wid) {
                        if phys.snap_x.is_some() {
                            // Floor 1 (Home Screen pager): Horizontal paging
                            let delta = -x * 30.0 - y * 30.0;
                            phys.apply_drag(delta);
                            phys.release_drag(delta * 4.0);
                        } else {
                            // Floor 2 (App Drawer): No horizontal motion exists; do NOT scroll without Shift!
                        }
                    }
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
