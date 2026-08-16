pub mod gestures;

use crate::core::Point;
use crate::core::render::draw::{
    find_clicked_button_with_scroll, find_hovered_button_with_scroll, find_hovered_scrollview,
};
use crate::core::ui::event::UiEvent;
use android_activity::InputStatus;
use android_activity::input::{InputEvent, MotionAction};

fn resolve_active_scroll(
    scroll_physics: &std::collections::HashMap<u64, super::app::physics_sync::ScrollPhysics>,
    id_opt: Option<&str>,
    default_x: f32,
    default_y: f32,
) -> (f32, f32) {
    if let Some(id_str) = id_opt {
        let sv_id = crate::core::ui::widget::fnv1a(id_str.as_bytes());
        if let Some(phys) = scroll_physics.get(&sv_id) {
            return (phys.pos_x, phys.pos_y);
        }
    }
    (default_x, default_y)
}

#[allow(clippy::too_many_lines)]
pub fn handle_input_event(
    input_event: &InputEvent,
    state: &mut super::app::AppState,
    root_element: &crate::core::types::Element,
    layout_tree: &crate::core::layout::LayoutNode,
) -> InputStatus {
    let get_max_scroll = |target_id: Option<u64>| -> f32 {
        target_id.and_then(|id| state.cached_max_scroll.get(&id).copied()).unwrap_or(0.0)
    };

    match input_event {
        InputEvent::MotionEvent(motion_event) => {
            let pointer = motion_event.pointer_at_index(motion_event.pointer_index());
            let point = Point::new(pointer.x(), pointer.y());

            let (delta_x, delta_y) = match motion_event.action() {
                MotionAction::Down | MotionAction::PointerDown => {
                    state.touch_start_pos = point;
                    state.last_touch_pos = point;
                    state.last_drag_delta = (0.0, 0.0);
                    state.total_touch_drag_distance = 0.0;
                    (0.0, 0.0)
                }
                _ => {
                    let raw_delta_x = -(point.x - state.last_touch_pos.x);
                    let raw_delta_y = -(point.y - state.last_touch_pos.y);
                    state.last_touch_pos = point;

                    let alpha = 0.85_f32;
                    let dx = alpha.mul_add(raw_delta_x, (1.0 - alpha) * state.last_drag_delta.0);
                    let dy = alpha.mul_add(raw_delta_y, (1.0 - alpha) * state.last_drag_delta.1);
                    state.total_touch_drag_distance += dx.abs() + dy.abs();
                    (dx, dy)
                }
            };

            match motion_event.action() {
                MotionAction::Down | MotionAction::Move | MotionAction::PointerDown => {
                    if motion_event.action() == MotionAction::Down
                        || motion_event.action() == MotionAction::PointerDown
                    {
                        if let Some((
                            crate::core::types::Element::ScrollView {
                                id, capture_drag, ..
                            },
                            _,
                        )) = find_hovered_scrollview(root_element, layout_tree, point)
                            && capture_drag.unwrap_or(true)
                        {
                            state.active_scrollview_drag = id
                                .as_deref()
                                .map(|id_str| crate::core::ui::widget::fnv1a(id_str.as_bytes()));
                        }
                        state.kinetic_scrolls.clear();
                        state.drag_history.clear();

                        if let Some((clicked_btn, _)) =
                            find_clicked_button_with_scroll(root_element, layout_tree, point, &|id_opt, sx, sy| resolve_active_scroll(&state.scroll_physics, id_opt, sx, sy))
                        {
                            state.event_bus.push(UiEvent::PointerDown(clicked_btn));
                        } else {
                            state.event_bus.push(UiEvent::PointerDown(0));
                        }
                    }

                    let prev_hovered = state.hovered_btn;
                    let hovered_data = find_hovered_button_with_scroll(root_element, layout_tree, point, &|id_opt, sx, sy| resolve_active_scroll(&state.scroll_physics, id_opt, sx, sy));
                    state.hovered_btn = hovered_data.map(|(id, _)| id);

                    let pointer_count = motion_event.pointers().count();
                    let gesture_name = match motion_event.action() {
                        MotionAction::Down | MotionAction::PointerDown => "TAP (Down)".to_string(),
                        MotionAction::Move => {
                            let dist = (point.x - state.touch_start_pos.x).hypot(point.y - state.touch_start_pos.y);
                            if dist > 16.0 * state.cached_density.0.max(1.0) { "SWIPE / DRAG".to_string() } else { "HOLD".to_string() }
                        }
                        _ => "TOUCH".to_string(),
                    };
                    let target_name = hovered_data.map(|(btn, _)| format!("btn_{btn:x}")).unwrap_or_else(|| "None".to_string());

                    if let Ok(mut prof) = state.profiler.lock() {
                        prof.touch_telemetry.active_pointers = pointer_count;
                        prof.touch_telemetry.gesture = gesture_name;
                        prof.touch_telemetry.target_element = target_name;
                        prof.touch_telemetry.touch_x = point.x;
                        prof.touch_telemetry.touch_y = point.y;
                    }

                    if prev_hovered != state.hovered_btn
                        && let Some(prev) = prev_hovered
                    {
                        state.event_bus.push(UiEvent::HoverEnd(prev));
                    }
                    if let Some((btn, rect)) = hovered_data {
                        state.event_bus.push(UiEvent::Hover(
                            btn,
                            rect.width,
                            rect.height,
                            point.x - rect.x,
                            point.y - rect.y,
                        ));
                    }
                    if motion_event.action() == MotionAction::Move {
                        state.last_drag_delta = (delta_x, delta_y);

                        state
                            .drag_history
                            .push_back((delta_x, delta_y, std::time::Instant::now()));
                        if state.drag_history.len() > 8 {
                            state.drag_history.pop_front();
                        }

                        if state.active_scrollview_drag.is_none() {
                            if let Some((
                                crate::core::types::Element::ScrollView {
                                    id, capture_drag, ..
                                },
                                _,
                            )) = find_hovered_scrollview(root_element, layout_tree, point)
                            {
                                if capture_drag.unwrap_or(true) {
                                    state.active_scrollview_drag = id.as_deref().map(|id_str| {
                                        crate::core::ui::widget::fnv1a(id_str.as_bytes())
                                    });
                                }
                            }
                        }

                        if let Some(sv_id) = state.active_scrollview_drag {
                            if let Some(phys) = state.scroll_physics.get_mut(&sv_id) {
                                phys.apply_drag(delta_x);
                                phys.is_dragging = true;
                                phys.snap_target_x = None;
                            } else {
                                state.event_bus.push(UiEvent::Scroll(
                                    Some(sv_id),
                                    delta_x,
                                    delta_y,
                                    999_999.0,
                                    get_max_scroll(Some(sv_id)),
                                ));
                            }
                        } else {
                            state.event_bus.push(UiEvent::Scroll(
                                state.hovered_btn,
                                delta_x,
                                delta_y,
                                999_999.0,
                                get_max_scroll(state.hovered_btn),
                            ));
                        }
                    }
                    InputStatus::Handled
                }
                MotionAction::Up | MotionAction::PointerUp => {
                    if let Some(sv_id) = state.active_scrollview_drag {
                        if state.scroll_physics.contains_key(&sv_id) {
                            let now = std::time::Instant::now();
                            let cutoff = now
                                .checked_sub(std::time::Duration::from_millis(150))
                                .unwrap_or(now);
                            let mut recent_count = 0;
                            let mut total_dx = 0.0;
                            let mut first_time = None;
                            let mut last_time = None;
                            
                            for &(dx, _, t) in state.drag_history.iter() {
                                if t >= cutoff {
                                    if first_time.is_none() {
                                        first_time = Some(t);
                                    }
                                    last_time = Some(t);
                                    total_dx += dx;
                                    recent_count += 1;
                                }
                            }

                            let vel_x = if recent_count >= 2 {
                                let dt = last_time.unwrap().duration_since(first_time.unwrap()).as_secs_f32().max(0.001);
                                total_dx / dt
                            } else if let Some(last) = last_time {
                                let dt = now.duration_since(last).as_secs_f32().max(0.001);
                                state.last_drag_delta.0 / dt
                            } else {
                                0.0
                            };

                            if let Some(phys) = state.scroll_physics.get_mut(&sv_id) {
                                phys.release_drag(vel_x);
                            }
                        } else {
                            let momentum_enabled = if let Some((
                                crate::core::types::Element::ScrollView {
                                    momentum_scrolling, ..
                                },
                                _,
                            )) =
                                find_hovered_scrollview(root_element, layout_tree, point)
                            {
                                momentum_scrolling.unwrap_or(true)
                            } else {
                                true
                            };

                            if momentum_enabled {
                                let cutoff = std::time::Instant::now()
                                    .checked_sub(std::time::Duration::from_millis(150))
                                    .unwrap_or_else(std::time::Instant::now);
                                let mut recent_count = 0;
                                let mut sum_dx = 0.0;
                                let mut sum_dy = 0.0;
                                
                                for &(dx, dy, t) in state.drag_history.iter() {
                                    if t >= cutoff {
                                        sum_dx += dx;
                                        sum_dy += dy;
                                        recent_count += 1;
                                    }
                                }

                                let (vel_x, vel_y) = if recent_count == 0 {
                                    state.last_drag_delta
                                } else {
                                    let n = recent_count as f32;
                                    (sum_dx / n, sum_dy / n)
                                };

                                state.kinetic_scrolls.push(super::app::KineticScroll {
                                    sv_id,
                                    velocity_x: vel_x,
                                    velocity_y: vel_y,
                                });
                            }
                        }
                    }

                    state.active_scrollview_drag = None;
                    state.last_drag_delta = (0.0, 0.0);
                    state.drag_history.clear();

                    let prev_hovered = state.hovered_btn;
                    let hovered_data = find_hovered_button_with_scroll(root_element, layout_tree, point, &|id_opt, sx, sy| resolve_active_scroll(&state.scroll_physics, id_opt, sx, sy));
                    state.hovered_btn = hovered_data.map(|(id, _)| id);
                    if let Some(prev) = prev_hovered {
                        state.event_bus.push(UiEvent::HoverEnd(prev));
                    }

                    state.event_bus.push(UiEvent::PointerUp(state.hovered_btn));

                    // Determine tap thresholds based on device density
                    let density = state.cached_density.0.max(1.0);
                    let tap_threshold = 48.0 * density;
                    let tap_dist = (point.x - state.touch_start_pos.x).hypot(point.y - state.touch_start_pos.y);
                    let is_static_tap = tap_dist < tap_threshold;
                    
                    let pointer_count = motion_event.pointers().count();
                    let gesture_name = if is_static_tap { "TAP (Release)".to_string() } else { "SWIPE (Release)".to_string() };
                    let target_name = hovered_data.map(|(btn, _)| format!("btn_{btn:x}")).unwrap_or_else(|| "None".to_string());

                    if let Ok(mut prof) = state.profiler.lock() {
                        prof.touch_telemetry.active_pointers = pointer_count;
                        prof.touch_telemetry.gesture = gesture_name;
                        prof.touch_telemetry.target_element = target_name;
                        prof.touch_telemetry.touch_x = point.x;
                        prof.touch_telemetry.touch_y = point.y;
                    }

                    if is_static_tap {
                        let clicked = find_clicked_button_with_scroll(root_element, layout_tree, state.touch_start_pos, &|id_opt, sx, sy| resolve_active_scroll(&state.scroll_physics, id_opt, sx, sy))
                            .or_else(|| find_clicked_button_with_scroll(root_element, layout_tree, point, &|id_opt, sx, sy| resolve_active_scroll(&state.scroll_physics, id_opt, sx, sy)));

                        if let Some((clicked_btn, rect)) = clicked {
                            crate::dev_log!("[Input] TAP on button id: {clicked_btn} (hex: {clicked_btn:x})");
                            state
                                .event_bus
                                .push(UiEvent::Click(clicked_btn, rect.width, rect.height));
                        } else {
                            crate::dev_log!("[Input] TAP clicked outside (no button at start={:?}, end={:?})", state.touch_start_pos, point);
                            state.event_bus.push(UiEvent::ClickOutside);
                        }
                    } else {
                        crate::dev_log!("[Input] Swipe detected (dist={tap_dist:.1}, thresh={tap_threshold:.1})");
                        state.event_bus.push(UiEvent::ClickOutside);
                    }
                    InputStatus::Handled
                }
                MotionAction::Cancel => {
                    state.active_scrollview_drag = None;
                    state.drag_history.clear();
                    let prev_hovered = state.hovered_btn;
                    state.hovered_btn = None;
                    if let Some(prev) = prev_hovered {
                        state.event_bus.push(UiEvent::HoverEnd(prev));
                    }
                    InputStatus::Handled
                }
                MotionAction::Scroll => {
                    let axis_v = pointer.axis_value(android_activity::input::Axis::Vscroll);
                    let axis_h = pointer.axis_value(android_activity::input::Axis::Hscroll);

                    if let Some((
                        crate::core::types::Element::ScrollView {
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
                            .map(|id_str| crate::core::ui::widget::fnv1a(id_str.as_bytes()));
                        state.event_bus.push(UiEvent::Scroll(
                            sv_id,
                            axis_h * -20.0 * factor,
                            axis_v * -20.0 * factor,
                            999_999.0,
                            get_max_scroll(sv_id),
                        ));
                    }
                    InputStatus::Handled
                }
                _ => InputStatus::Unhandled,
            }
        }
        InputEvent::KeyEvent(key_event) => {
            if key_event.action() == android_activity::input::KeyAction::Down {
                let keycode = key_event.key_code();
                if keycode == android_activity::input::Keycode::Del {
                    state.event_bus.push(UiEvent::Backspace);
                } else if let Some(c) = gestures::keycode_to_char(keycode) {
                    state.event_bus.push(UiEvent::TextInput(c.to_string()));
                }
            }
            InputStatus::Handled
        }
        _ => InputStatus::Unhandled,
    }
}
