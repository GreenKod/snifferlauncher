pub mod release;
pub mod scroll;

pub use scroll::resolve_active_scroll;

use android_activity::InputStatus;
use android_activity::input::{MotionAction, MotionEvent};
use sniffer_core::Point;

pub fn handle_motion_event(
    motion_event: &MotionEvent,
    state: &mut crate::app::AppState,
    root_element: &sniffer_core::types::Element,
    layout_tree: &sniffer_core::layout::LayoutNode,
) -> InputStatus {
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
            scroll::handle_touch_move_or_down(
                motion_event,
                state,
                root_element,
                layout_tree,
                point,
                delta_x,
                delta_y,
            )
        }
        MotionAction::Up | MotionAction::PointerUp => {
            release::handle_touch_release(motion_event, state, root_element, layout_tree, point)
        }
        MotionAction::Cancel => release::handle_touch_cancel(state),
        MotionAction::Scroll => release::handle_touch_wheel_scroll(
            motion_event,
            state,
            root_element,
            layout_tree,
            point,
        ),
        _ => InputStatus::Unhandled,
    }
}
