use crate::core::Point;
use crate::core::ui::event::UiEvent;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

#[allow(clippy::missing_panics_doc)]
pub fn handle_event(
    event: Event,
    app: &mut super::app::AppState,
    clicked_pos: &mut Option<Point>,
    mouse_released: &mut bool,
    mouse_moved: &mut bool,
    scroll_events: &mut Vec<(i32, i32)>,
    drag_events: &mut Vec<(i32, i32)>,
) {
    match event {
        Event::Quit { .. }
        | Event::KeyDown {
            keycode: Some(Keycode::Escape),
            ..
        } => {
            app.running = false;
        }
        Event::KeyDown {
            keycode: Some(Keycode::Backspace),
            ..
        } => {
            app.event_bus.push(UiEvent::Backspace);
        }
        Event::MouseMotion {
            x,
            y,
            xrel,
            yrel,
            mousestate,
            ..
        } => {
            app.last_mouse_pos = Point::new(
                f32::from(i16::try_from(x).expect("mouse x fits in i16")),
                f32::from(i16::try_from(y).expect("mouse y fits in i16")),
            );
            *mouse_moved = true;
            if mousestate.left() {
                drag_events.push((-xrel, -yrel)); // Invert to simulate touch panning
            }
        }
        Event::MouseButtonDown {
            mouse_btn: sdl2::mouse::MouseButton::Left,
            x,
            y,
            ..
        } => {
            *clicked_pos = Some(Point::new(
                f32::from(i16::try_from(x).expect("mouse x fits in i16")),
                f32::from(i16::try_from(y).expect("mouse y fits in i16")),
            ));
        }
        Event::MouseButtonUp {
            mouse_btn: sdl2::mouse::MouseButton::Left,
            ..
        } => {
            *mouse_released = true;
        }
        Event::Window {
            win_event: sdl2::event::WindowEvent::Leave,
            ..
        } => {
            let prev_hovered = app.hovered_btn;
            app.hovered_btn = None;
            if let Some(prev) = prev_hovered {
                app.event_bus.push(UiEvent::HoverEnd(prev));
            }
            app.last_mouse_pos = Point::new(-9999.0, -9999.0);
            *mouse_moved = true;
        }
        Event::TextInput { text, .. } => {
            app.event_bus.push(UiEvent::TextInput(text));
        }
        Event::MouseWheel { x, y, .. } => {
            scroll_events.push((x, y));
        }
        _ => {}
    }
}
