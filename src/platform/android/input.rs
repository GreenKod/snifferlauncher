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
        target_id.map_or(0.0, |target| {
            let mut found_max = 0.0;
            let mut search = vec![(root_element, layout_tree)];
            while let Some((el, lay)) = search.pop() {
                if let crate::core::types::Element::ScrollView { id, .. } = el
                    && id
                        .as_deref()
                        .map(|s| crate::core::ui::widget::fnv1a(s.as_bytes()))
                        == Some(target)
                {
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
                    if min_y <= max_y {
                        found_max = (max_y - min_y - view_height).max(0.0);
                    }
                    break;
                }
                if let crate::core::types::Element::Container { children, .. }
                | crate::core::types::Element::ScrollView { children, .. } = el
                {
                    for (child, child_lay) in children.iter().zip(lay.children.iter()) {
                        search.push((child, child_lay));
                    }
                }
            }
            found_max
        })
    };

    match input_event {
        InputEvent::MotionEvent(motion_event) => {
            let pointer = motion_event.pointer_at_index(motion_event.pointer_index());
            let point = Point::new(pointer.raw_x(), pointer.raw_y());
            let delta_x = -(point.x - state.last_touch_pos.x);
            let delta_y = -(point.y - state.last_touch_pos.y);
            state.last_touch_pos = point;

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
                        if let Some(sv_id) = state.active_scrollview_drag {
                            state.event_bus.push(UiEvent::Scroll(
                                Some(sv_id),
                                delta_x,
                                delta_y,
                                999_999.0,
                                get_max_scroll(Some(sv_id)),
                            ));
                        } else if let Some((
                            crate::core::types::Element::ScrollView {
                                id, capture_drag, ..
                            },
                            _,
                        )) = find_hovered_scrollview(root_element, layout_tree, point)
                        {
                            if capture_drag.unwrap_or(true)
                                && state.active_scrollview_drag.is_none()
                            {
                                state.active_scrollview_drag = id.as_deref().map(|id_str| {
                                    crate::core::ui::widget::fnv1a(id_str.as_bytes())
                                });
                            }
                            let sv_id = id
                                .as_deref()
                                .map(|id_str| crate::core::ui::widget::fnv1a(id_str.as_bytes()));
                            state.event_bus.push(UiEvent::Scroll(
                                sv_id,
                                delta_x,
                                delta_y,
                                999_999.0,
                                get_max_scroll(sv_id),
                            ));
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
                    if let Some(sv_id) = state.active_scrollview_drag
                        && (state.last_drag_delta.0.abs() > 0.5
                            || state.last_drag_delta.1.abs() > 0.5)
                    {
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
                            state.kinetic_scrolls.push(super::app::KineticScroll {
                                sv_id,
                                velocity_x: state.last_drag_delta.0,
                                velocity_y: state.last_drag_delta.1,
                            });
                        }
                    }

                    state.active_scrollview_drag = None;
                    state.last_drag_delta = (0.0, 0.0);
                    let prev_hovered = state.hovered_btn;
                    let hovered_data = find_hovered_button(root_element, layout_tree, point);
                    state.hovered_btn = hovered_data.map(|(id, _)| id);
                    if let Some(prev) = prev_hovered {
                        state.event_bus.push(UiEvent::HoverEnd(prev));
                    }

                    state.event_bus.push(UiEvent::PointerUp(state.hovered_btn));

                    if let Some((clicked_btn, rect)) =
                        find_clicked_button(root_element, layout_tree, point)
                    {
                        state
                            .event_bus
                            .push(UiEvent::Click(clicked_btn, rect.width, rect.height));
                    } else {
                        state.event_bus.push(UiEvent::ClickOutside);
                    }
                    InputStatus::Handled
                }
                MotionAction::Cancel => {
                    state.active_scrollview_drag = None;
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
