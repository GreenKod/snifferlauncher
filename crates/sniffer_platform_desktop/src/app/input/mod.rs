pub mod drag;
pub mod mouse;
pub mod pointer_up;
pub mod scroll;

use super::state::{AppState, FrameInputState};
use sniffer_core::Point;
use sniffer_core::layout::LayoutNode;
use sniffer_core::types::Element;

/// Master frame input coordinator for desktop window interactions.
pub fn process_frame_inputs(
    app: &mut AppState,
    input: &FrameInputState,
    root_element: &Element,
    layout_tree: &LayoutNode,
    scaled_last_mouse_pos: Point,
    scaled_clicked_pos: Option<Point>,
    scale_x: f32,
    scale_y: f32,
) {
    if input.mouse_moved {
        mouse::handle_mouse_move(app, root_element, layout_tree, scaled_last_mouse_pos);
    }

    if let Some(clicked_pt) = scaled_clicked_pos {
        mouse::handle_mouse_down(app, root_element, layout_tree, clicked_pt);
    }

    if !input.drag_events.is_empty() {
        drag::handle_drag_events(
            app,
            &input.drag_events,
            root_element,
            layout_tree,
            scaled_last_mouse_pos,
            scale_x,
            scale_y,
        );
    }

    if input.mouse_released {
        pointer_up::handle_mouse_release(app, root_element, layout_tree, scaled_last_mouse_pos);
    }

    if !input.scroll_events.is_empty() {
        scroll::handle_scroll_events(
            app,
            &input.scroll_events,
            root_element,
            layout_tree,
            scaled_last_mouse_pos,
        );
    }

    scroll::tick_kinetic_scrolls(app, root_element, layout_tree);
}
