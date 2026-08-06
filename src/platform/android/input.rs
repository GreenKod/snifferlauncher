use crate::core::Point;
use crate::core::render::draw::{
    find_clicked_button, find_hovered_button, find_hovered_scrollview,
};
use crate::core::ui::event::UiEvent;
use android_activity::InputStatus;
use android_activity::input::{InputEvent, MotionAction};

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
                    state.last_touch_pos = point;
                    state.last_drag_delta = (0.0, 0.0);
                    state.total_touch_drag_distance = 0.0;
                    (0.0, 0.0)
                }
                _ => {
                    let raw_delta_x = -(point.x - state.last_touch_pos.x);
                    let raw_delta_y = -(point.y - state.last_touch_pos.y);
                    state.last_touch_pos = point;

                    // Exponential moving average (EMA) smoothing for high-responsiveness 1:1 touch tracking.
                    // alpha=0.85: eliminates input lag while rejecting digitizer noise.
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
                        // FIX (Sorun 3): Yeni dokunuşta geçmiş velocity tamponunu sıfırla
                        state.kinetic_scrolls.clear();
                        state.drag_history.clear();

                        if let Some((clicked_btn, _)) =
                            find_clicked_button(root_element, layout_tree, point)
                        {
                            state.event_bus.push(UiEvent::PointerDown(clicked_btn));
                        } else {
                            state.event_bus.push(UiEvent::PointerDown(0));
                        }
                    }

                    let prev_hovered = state.hovered_btn;
                    let hovered_data = find_hovered_button(root_element, layout_tree, point);
                    state.hovered_btn = hovered_data.map(|(id, _)| id);

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

                        // FIX (Sorun 3): Velocity geçmişini kaydet (son ~8 frame).
                        // Parmak yavaşlayarak bırakıldığında son tek delta yerine
                        // rolling average kullanarak tutarlı momentum sağlar.
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
                            // Rust-managed mı? Varsa fizik motoruna doğrudan uygula, JS'i bypass et
                            if let Some(phys) = state.scroll_physics.get_mut(&sv_id) {
                                phys.apply_drag(delta_x); // Yatay pager: sadece X
                                phys.is_dragging = true;
                                phys.snap_target_x = None; // Sürüklerken snap'i iptal et
                            } else {
                                // Klasik ScrollView: JS'e event gönder
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
                        // Rust-managed mı? Varsa velocity'yi fizik motoruna ver
                        if state.scroll_physics.contains_key(&sv_id) {
                            let now = std::time::Instant::now();
                            let cutoff = now
                                .checked_sub(std::time::Duration::from_millis(150))
                                .unwrap_or(now);
                            let recent: Vec<_> = state
                                .drag_history
                                .iter()
                                .filter(|(_, _, t)| *t >= cutoff)
                                .collect();

                            // Frame-rate bağımsız dinamik fling hız hesabı (v = dx / dt)
                            // max(0.001) sıfıra bölünmeyi ve yüksek digitizer yenileme gürültüsünü önler.
                            let vel_x = if recent.len() >= 2 {
                                let first = recent.first().unwrap();
                                let last = recent.last().unwrap();
                                let dt = last.2.duration_since(first.2).as_secs_f32().max(0.001);
                                let total_dx: f32 = recent.iter().map(|(dx, _, _)| *dx).sum();
                                total_dx / dt
                            } else if let Some(last) = recent.last() {
                                let dt = now.duration_since(last.2).as_secs_f32().max(0.001);
                                state.last_drag_delta.0 / dt
                            } else {
                                0.0
                            };

                            if let Some(phys) = state.scroll_physics.get_mut(&sv_id) {
                                phys.release_drag(vel_x);
                            }
                            // Kinetic scroll ve JS event yok: fizik motoru devretti
                        } else {
                            // Klasik ScrollView: eski momentum sistemi
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
                                // Use last 150ms of drag history for fling velocity
                                let cutoff = std::time::Instant::now()
                                    .checked_sub(std::time::Duration::from_millis(150))
                                    .unwrap_or_else(std::time::Instant::now);
                                let recent: Vec<_> = state
                                    .drag_history
                                    .iter()
                                    .filter(|(_, _, t)| *t >= cutoff)
                                    .collect();

                                let (vel_x, vel_y) = if recent.is_empty() {
                                    state.last_drag_delta
                                } else {
                                    let n = recent.len() as f32;
                                    let vx = recent.iter().map(|(dx, _, _)| *dx).sum::<f32>() / n;
                                    let vy = recent.iter().map(|(_, dy, _)| *dy).sum::<f32>() / n;
                                    (vx, vy)
                                };

                                // Always push kinetic scroll — velocity gate is handled by retain_mut
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
                    let hovered_data = find_hovered_button(root_element, layout_tree, point);
                    state.hovered_btn = hovered_data.map(|(id, _)| id);
                    if let Some(prev) = prev_hovered {
                        state.event_bus.push(UiEvent::HoverEnd(prev));
                    }

                    state.event_bus.push(UiEvent::PointerUp(state.hovered_btn));

                    // Gate Click event on touch slop (< 12.0px drag distance) so scrolling/swiping never launches apps
                    let is_static_tap = state.total_touch_drag_distance < 12.0;
                    if is_static_tap {
                        if let Some((clicked_btn, rect)) =
                            find_clicked_button(root_element, layout_tree, point)
                        {
                            state
                                .event_bus
                                .push(UiEvent::Click(clicked_btn, rect.width, rect.height));
                        } else {
                            state.event_bus.push(UiEvent::ClickOutside);
                        }
                    } else {
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
                use android_activity::input::Keycode;
                let keycode = key_event.key_code();
                if keycode == Keycode::Del {
                    state.event_bus.push(UiEvent::Backspace);
                } else {
                    let ch = match keycode {
                        Keycode::A => Some('a'),
                        Keycode::B => Some('b'),
                        Keycode::C => Some('c'),
                        Keycode::D => Some('d'),
                        Keycode::E => Some('e'),
                        Keycode::F => Some('f'),
                        Keycode::G => Some('g'),
                        Keycode::H => Some('h'),
                        Keycode::I => Some('i'),
                        Keycode::J => Some('j'),
                        Keycode::K => Some('k'),
                        Keycode::L => Some('l'),
                        Keycode::M => Some('m'),
                        Keycode::N => Some('n'),
                        Keycode::O => Some('o'),
                        Keycode::P => Some('p'),
                        Keycode::Q => Some('q'),
                        Keycode::R => Some('r'),
                        Keycode::S => Some('s'),
                        Keycode::T => Some('t'),
                        Keycode::U => Some('u'),
                        Keycode::V => Some('v'),
                        Keycode::W => Some('w'),
                        Keycode::X => Some('x'),
                        Keycode::Y => Some('y'),
                        Keycode::Z => Some('z'),
                        Keycode::Keycode0 => Some('0'),
                        Keycode::Keycode1 => Some('1'),
                        Keycode::Keycode2 => Some('2'),
                        Keycode::Keycode3 => Some('3'),
                        Keycode::Keycode4 => Some('4'),
                        Keycode::Keycode5 => Some('5'),
                        Keycode::Keycode6 => Some('6'),
                        Keycode::Keycode7 => Some('7'),
                        Keycode::Keycode8 => Some('8'),
                        Keycode::Keycode9 => Some('9'),
                        Keycode::Space => Some(' '),
                        Keycode::Period => Some('.'),
                        Keycode::Comma => Some(','),
                        Keycode::Minus => Some('-'),
                        _ => None,
                    };
                    if let Some(c) = ch {
                        state.event_bus.push(UiEvent::TextInput(c.to_string()));
                    }
                }
            }
            InputStatus::Handled
        }
        _ => InputStatus::Unhandled,
    }
}
