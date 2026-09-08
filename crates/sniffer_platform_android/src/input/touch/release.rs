use super::scroll::resolve_active_scroll;
use android_activity::InputStatus;
use android_activity::input::MotionEvent;
use sniffer_core::Point;
use sniffer_core::ui::event::UiEvent;
use sniffer_render::draw::{
    find_clicked_button_with_scroll, find_hovered_button_with_scroll, find_hovered_scrollview,
};

#[allow(clippy::too_many_lines)]
pub fn handle_touch_release(
    motion_event: &MotionEvent,
    state: &mut crate::app::AppState,
    root_element: &sniffer_core::types::Element,
    layout_tree: &sniffer_core::layout::LayoutNode,
    point: Point,
) -> InputStatus {
    let cutoff = std::time::Instant::now()
        .checked_sub(std::time::Duration::from_millis(150))
        .unwrap_or_else(std::time::Instant::now);
    let mut recent_count = 0;
    let mut total_dx = 0.0;
    let mut first_time = None;
    let mut last_time = None;
    let mut total_dy = 0.0;

    for &(dx, dy, t) in &state.drag_history {
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

    let (fling_vel_x, fling_vel_y) = if recent_count >= 2 {
        let dt_recent = last_time
            .unwrap()
            .duration_since(first_time.unwrap())
            .as_secs_f32()
            .max(0.001);
        (total_dx / dt_recent, total_dy / dt_recent)
    } else if let Some(last) = last_time {
        let dt_recent = std::time::Instant::now()
            .duration_since(last)
            .as_secs_f32()
            .max(0.001);
        (
            state.last_drag_delta.0 / dt_recent,
            state.last_drag_delta.1 / dt_recent,
        )
    } else {
        (0.0, 0.0)
    };

    if let Some(sv_id) = state.active_scrollview_drag {
        if let Some(phys) = state.scroll_physics.get_mut(&sv_id) {
            if phys.snap_x.is_some() {
                phys.release_drag(fling_vel_x);
            } else {
                phys.release_drag_y(fling_vel_y);
            }
        } else {
            let momentum_enabled = if let Some((
                sniffer_core::types::Element::ScrollView {
                    momentum_scrolling, ..
                },
                _,
            )) = find_hovered_scrollview(root_element, layout_tree, point)
            {
                momentum_scrolling.unwrap_or(true)
            } else {
                true
            };

            if momentum_enabled {
                state.kinetic_scrolls.push(crate::app::KineticScroll {
                    sv_id,
                    velocity_x: fling_vel_x,
                    velocity_y: fling_vel_y,
                });
            }
        }
    }

    state.active_scrollview_drag = None;
    state.last_drag_delta = (0.0, 0.0);
    state.drag_history.clear();

    let prev_hovered = state.hovered_btn;
    let hovered_data =
        find_hovered_button_with_scroll(root_element, layout_tree, point, &|id_opt, sx, sy| {
            resolve_active_scroll(&state.scroll_physics, id_opt, sx, sy)
        });
    state.hovered_btn = hovered_data.map(|(id, _)| id);
    if let Some(prev) = prev_hovered {
        state.event_bus.push(UiEvent::HoverEnd(prev));
    }

    state.event_bus.push(UiEvent::PointerUp(state.hovered_btn));

    let density = state.cached_density.0.max(1.0);
    let tap_threshold = 48.0 * density;
    let tap_dist = (point.x - state.touch_start_pos.x).hypot(point.y - state.touch_start_pos.y);
    let is_static_tap = tap_dist < tap_threshold;

    let pointer_count = motion_event.pointers().count();
    let gesture_name = if is_static_tap {
        "TAP (Release)".to_string()
    } else {
        "SWIPE (Release)".to_string()
    };
    let target_name =
        hovered_data.map_or_else(|| "None".to_string(), |(btn, _)| format!("btn_{btn:x}"));

    if let Ok(mut prof) = state.profiler.lock() {
        prof.touch_telemetry.active_pointers = pointer_count;
        prof.touch_telemetry.gesture = gesture_name;
        prof.touch_telemetry.target_element = target_name;
        prof.touch_telemetry.touch_x = point.x;
        prof.touch_telemetry.touch_y = point.y;
    }

    if is_static_tap {
        let clicked = find_clicked_button_with_scroll(
            root_element,
            layout_tree,
            state.touch_start_pos,
            &|id_opt, sx, sy| resolve_active_scroll(&state.scroll_physics, id_opt, sx, sy),
        )
        .or_else(|| {
            find_clicked_button_with_scroll(root_element, layout_tree, point, &|id_opt, sx, sy| {
                resolve_active_scroll(&state.scroll_physics, id_opt, sx, sy)
            })
        });

        if let Some((clicked_btn, rect)) = clicked {
            sniffer_core::dev_log!(
                "[Input] TAP on button id: {clicked_btn} (hex: {clicked_btn:x})"
            );
            state
                .event_bus
                .push(UiEvent::Click(clicked_btn, rect.width, rect.height));
        } else {
            sniffer_core::dev_log!(
                "[Input] TAP clicked outside (no button at start={:?}, end={:?})",
                state.touch_start_pos,
                point
            );
            state.event_bus.push(UiEvent::ClickOutside);
        }
    } else {
        let dy = point.y - state.touch_start_pos.y;
        let dx = point.x - state.touch_start_pos.x;
        let was_scrolled_down = state.active_scrollview_start_y > 10.0;
        let vel_y_down = -fling_vel_y;

        let swipe_thresh = 120.0 * density;
        let flick_thresh = 50.0 * density;

        if dy.abs() > dx.abs() * 1.3 {
            if dy < 0.0 {
                // SwipeUp: Open drawer from home screen
                let upward_dist = -dy;
                let is_flick = upward_dist > flick_thresh && vel_y_down < -400.0;
                let is_drag = upward_dist >= swipe_thresh;
                if is_flick || is_drag {
                    state.event_bus.push(UiEvent::SwipeUp);
                }
            } else if !was_scrolled_down {
                // SwipeDown: Close drawer back to home screen
                // Only allowed when NOT scrolled down within the list of apps!
                let downward_dist = dy;
                let is_flick = downward_dist > flick_thresh && vel_y_down > 400.0;
                let is_drag = downward_dist >= (swipe_thresh * 1.1);
                if is_flick || is_drag {
                    state.event_bus.push(UiEvent::SwipeDown);
                }
            }
        }
        state.event_bus.push(UiEvent::ClickOutside);
    }
    state.active_scrollview_start_y = 0.0;
    InputStatus::Handled
}

pub fn handle_touch_cancel(state: &mut crate::app::AppState) -> InputStatus {
    state.active_scrollview_drag = None;
    state.active_scrollview_start_y = 0.0;
    state.drag_history.clear();
    let prev_hovered = state.hovered_btn;
    state.hovered_btn = None;
    if let Some(prev) = prev_hovered {
        state.event_bus.push(UiEvent::HoverEnd(prev));
    }
    InputStatus::Handled
}

pub fn handle_touch_wheel_scroll(
    motion_event: &MotionEvent,
    state: &mut crate::app::AppState,
    root_element: &sniffer_core::types::Element,
    layout_tree: &sniffer_core::layout::LayoutNode,
    point: Point,
) -> InputStatus {
    let pointer = motion_event.pointer_at_index(motion_event.pointer_index());
    let axis_v = pointer.axis_value(android_activity::input::Axis::Vscroll);
    let axis_h = pointer.axis_value(android_activity::input::Axis::Hscroll);

    if let Some((
        sniffer_core::types::Element::ScrollView {
            id,
            scroll_sensitivity,
            dynamic_sensitivity,
            ..
        },
        lay,
    )) = find_hovered_scrollview(root_element, layout_tree, point)
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
        state.event_bus.push(UiEvent::Scroll(
            sv_id,
            axis_h * -20.0 * factor,
            axis_v * -20.0 * factor,
            999_999.0,
            state.get_max_scroll(sv_id),
        ));
    }
    InputStatus::Handled
}
