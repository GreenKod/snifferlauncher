use super::super::helpers::resolve_active_scroll;
use super::super::state::{AppState, KineticScroll};
use sniffer_core::Point;
use sniffer_core::layout::LayoutNode;
use sniffer_core::types::Element;
use sniffer_core::ui::event::UiEvent;
use sniffer_render::draw::{find_clicked_button_with_scroll, find_hovered_button_with_scroll};

/// Processes mouse button release, tap threshold determination, and kinetic inertia.
#[allow(clippy::cast_precision_loss)]
pub fn handle_mouse_release(
    app: &mut AppState,
    root_element: &Element,
    layout_tree: &LayoutNode,
    scaled_last_mouse_pos: Point,
) {
    let now = std::time::Instant::now();
    let cutoff = now
        .checked_sub(std::time::Duration::from_millis(150))
        .unwrap_or(now);
    let mut recent_count = 0;
    let mut total_dx = 0.0;
    let mut total_dy = 0.0;
    let mut first_time = None;
    let mut last_time = None;

    for &(dx, dy, t) in &app.drag_history {
        if t >= cutoff {
            if first_time.is_none() {
                first_time = Some(t);
            }
            last_time = Some(t);
            total_dx += dx;
            total_dy += dy;
            recent_count += 1;
        }
    }

    let (vel_x, vel_y) = if recent_count >= 2 {
        let dt_recent = last_time
            .unwrap()
            .duration_since(first_time.unwrap())
            .as_secs_f32()
            .max(0.001);
        (total_dx / dt_recent, total_dy / dt_recent)
    } else if let Some(last) = last_time {
        let dt_recent = now.duration_since(last).as_secs_f32().max(0.001);
        (
            app.last_drag_delta.0 / dt_recent,
            app.last_drag_delta.1 / dt_recent,
        )
    } else {
        (0.0, 0.0)
    };

    if let Some(sv_id) = app.active_scrollview_drag {
        if let Some(phys) = app.scroll_physics.get_mut(&sv_id) {
            if phys.snap_x.is_some() {
                phys.release_drag(vel_x);
            } else {
                phys.release_drag_y(vel_y);
            }
        } else {
            let momentum_enabled = if let Some((
                Element::ScrollView {
                    momentum_scrolling, ..
                },
                _,
            )) = sniffer_render::draw::find_hovered_scrollview(
                root_element,
                layout_tree,
                scaled_last_mouse_pos,
            ) {
                momentum_scrolling.unwrap_or(true)
            } else {
                true
            };

            if momentum_enabled {
                app.kinetic_scrolls.push(KineticScroll {
                    sv_id,
                    velocity_x: app.last_drag_delta.0,
                    velocity_y: app.last_drag_delta.1,
                });
            }
        }
    }

    let released_btn = find_hovered_button_with_scroll(
        root_element,
        layout_tree,
        scaled_last_mouse_pos,
        &|id_opt, sx, sy| resolve_active_scroll(&app.scroll_physics, id_opt, sx, sy),
    )
    .map(|(id, _)| id);
    app.event_bus.push(UiEvent::PointerUp(released_btn));

    if app.total_drag_dist < 15.0 {
        if let Some(down_pt) = app.mouse_down_pos {
            if let Some((clicked_btn, rect)) = find_clicked_button_with_scroll(
                root_element,
                layout_tree,
                down_pt,
                &|id_opt, sx, sy| resolve_active_scroll(&app.scroll_physics, id_opt, sx, sy),
            )
            .or_else(|| {
                find_clicked_button_with_scroll(
                    root_element,
                    layout_tree,
                    scaled_last_mouse_pos,
                    &|id_opt, sx, sy| resolve_active_scroll(&app.scroll_physics, id_opt, sx, sy),
                )
            }) {
                app.event_bus
                    .push(UiEvent::Click(clicked_btn, rect.width, rect.height));
            } else {
                app.event_bus.push(UiEvent::ClickOutside);
            }
        }
    } else if let Some(down_pt) = app.mouse_down_pos {
        let dy = scaled_last_mouse_pos.y - down_pt.y;
        let dx = scaled_last_mouse_pos.x - down_pt.x;
        let was_scrolled_down = app.active_scrollview_start_y > 10.0;
        let vel_y_down = -vel_y;

        if dy.abs() > dx.abs() * 1.3 {
            if dy < 0.0 {
                // SwipeUp: Open drawer from home screen
                let upward_dist = -dy;
                let is_flick = upward_dist > 45.0 && vel_y_down < -350.0;
                let is_drag = upward_dist >= 100.0;
                if is_flick || is_drag {
                    app.event_bus.push(UiEvent::SwipeUp);
                }
            } else if !was_scrolled_down {
                // SwipeDown: Close drawer back to home screen
                // Only allowed when NOT scrolled down within the list of apps!
                let downward_dist = dy;
                let is_flick = downward_dist > 50.0 && vel_y_down > 350.0;
                let is_drag = downward_dist >= 120.0;
                if is_flick || is_drag {
                    app.event_bus.push(UiEvent::SwipeDown);
                }
            }
        }
    }

    app.active_scrollview_drag = None;
    app.active_scrollview_start_y = 0.0;
    app.last_drag_delta = (0.0, 0.0);
    app.drag_history.clear();
    app.mouse_down_pos = None;
}
