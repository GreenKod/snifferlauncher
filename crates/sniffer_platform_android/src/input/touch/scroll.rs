use android_activity::InputStatus;
use android_activity::input::{MotionAction, MotionEvent};
use sniffer_core::Point;
use sniffer_core::ui::event::UiEvent;
use sniffer_render::draw::{
    find_clicked_button_with_scroll, find_hovered_button_with_scroll, find_hovered_scrollview,
};

#[allow(clippy::implicit_hasher)]
pub fn resolve_active_scroll(
    scroll_physics: &std::collections::HashMap<u64, crate::app::physics_sync::ScrollPhysics>,
    id_opt: Option<&str>,
    default_x: f32,
    default_y: f32,
) -> (f32, f32) {
    if let Some(id_str) = id_opt {
        let sv_id = sniffer_core::ui::widget::fnv1a(id_str.as_bytes());
        if let Some(phys) = scroll_physics.get(&sv_id) {
            return (phys.pos_x, phys.pos_y);
        }
    }
    (default_x, default_y)
}

#[allow(clippy::too_many_lines)]
pub fn handle_touch_move_or_down(
    motion_event: &MotionEvent,
    state: &mut crate::app::AppState,
    root_element: &sniffer_core::types::Element,
    layout_tree: &sniffer_core::layout::LayoutNode,
    point: Point,
    delta_x: f32,
    delta_y: f32,
) -> InputStatus {
    if motion_event.action() == MotionAction::Down
        || motion_event.action() == MotionAction::PointerDown
    {
        if let Some((
            sniffer_core::types::Element::ScrollView {
                id, capture_drag, ..
            },
            _,
        )) = find_hovered_scrollview(root_element, layout_tree, point)
            && capture_drag.unwrap_or(true)
        {
            state.active_scrollview_drag = id
                .as_deref()
                .map(|id_str| sniffer_core::ui::widget::fnv1a(id_str.as_bytes()));
        }
        state.kinetic_scrolls.clear();
        state.drag_history.clear();

        if let Some((clicked_btn, _)) =
            find_clicked_button_with_scroll(root_element, layout_tree, point, &|id_opt, sx, sy| {
                resolve_active_scroll(&state.scroll_physics, id_opt, sx, sy)
            })
        {
            state.event_bus.push(UiEvent::PointerDown(clicked_btn));
        } else {
            state.event_bus.push(UiEvent::PointerDown(0));
        }
    }

    let prev_hovered = state.hovered_btn;
    let hovered_data =
        find_hovered_button_with_scroll(root_element, layout_tree, point, &|id_opt, sx, sy| {
            resolve_active_scroll(&state.scroll_physics, id_opt, sx, sy)
        });
    state.hovered_btn = hovered_data.map(|(id, _)| id);

    match (prev_hovered, hovered_data) {
        (Some(prev), Some((current, rect))) if prev != current => {
            state.event_bus.push(UiEvent::HoverEnd(prev));
            state.event_bus.push(UiEvent::Hover(
                current,
                rect.width,
                rect.height,
                point.x - rect.x,
                point.y - rect.y,
            ));
        }
        (Some(prev), Some((current, rect))) if prev == current => {
            state.event_bus.push(UiEvent::Hover(
                current,
                rect.width,
                rect.height,
                point.x - rect.x,
                point.y - rect.y,
            ));
        }
        (None, Some((current, rect))) => {
            state.event_bus.push(UiEvent::Hover(
                current,
                rect.width,
                rect.height,
                point.x - rect.x,
                point.y - rect.y,
            ));
        }
        (Some(prev), None) => {
            state.event_bus.push(UiEvent::HoverEnd(prev));
        }
        _ => {}
    }

    if motion_event.action() == MotionAction::Move {
        state.last_drag_delta = (delta_x, delta_y);
        state
            .drag_history
            .push_back((delta_x, delta_y, std::time::Instant::now()));
        if state.drag_history.len() > 16 {
            state.drag_history.pop_front();
        }

        if state.active_scrollview_drag.is_none() {
            if let Some((
                sniffer_core::types::Element::ScrollView {
                    id, capture_drag, ..
                },
                _,
            )) = find_hovered_scrollview(root_element, layout_tree, point)
            {
                if capture_drag.unwrap_or(true) {
                    state.active_scrollview_drag = id
                        .as_deref()
                        .map(|id_str| sniffer_core::ui::widget::fnv1a(id_str.as_bytes()));
                }
            }
        }

        if let Some(sv_id) = state.active_scrollview_drag {
            let phys = state.scroll_physics.entry(sv_id).or_default();
            phys.apply_drag(delta_x);
            phys.is_dragging = true;
            phys.snap_target_x = None;
        }
    }

    let pointer_count = motion_event.pointers().count();
    let gesture_name = match motion_event.action() {
        MotionAction::Down | MotionAction::PointerDown => "DOWN",
        MotionAction::Move => {
            if pointer_count > 1 {
                "MULTI_TOUCH / PINCH"
            } else {
                "DRAG / PAN"
            }
        }
        _ => "UNKNOWN",
    }
    .to_string();

    let target_name =
        hovered_data.map_or_else(|| "None".to_string(), |(btn, _)| format!("btn_{btn:x}"));

    if let Ok(mut prof) = state.profiler.lock() {
        prof.touch_telemetry.active_pointers = pointer_count;
        prof.touch_telemetry.gesture = gesture_name;
        prof.touch_telemetry.target_element = target_name;
        prof.touch_telemetry.touch_x = point.x;
        prof.touch_telemetry.touch_y = point.y;
    }

    InputStatus::Handled
}
