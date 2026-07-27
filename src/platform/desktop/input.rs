use crate::core::Point;
use crate::core::ui::event::UiEvent;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, Ime, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{Key, NamedKey};

#[allow(
    clippy::missing_panics_doc,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)]
pub fn handle_winit_event(
    event: &WindowEvent,
    app: &mut super::app::AppState,
    input: &mut super::app::FrameInputState,
) {
    match event {
        WindowEvent::CloseRequested => {
            app.running = false;
        }
        WindowEvent::Focused(focused) => {
            app.window_focused = *focused;
            if !*focused {
                if let Some(prev) = app.hovered_btn.take() {
                    app.event_bus.push(UiEvent::HoverEnd(prev));
                }
                app.last_mouse_pos = Point::new(-9999.0, -9999.0);
                input.mouse_moved = true;
            }
        }
        WindowEvent::Ime(ime_event) => match ime_event {
            Ime::Commit(text) => {
                app.event_bus.push(UiEvent::TextInput(text.clone()));
            }
            Ime::Preedit(_text, _cursor) => {
                // Preedit candidate string received (can be used for inline IME UI preview if needed)
            }
            _ => {}
        },
        WindowEvent::KeyboardInput { event, .. } => {
            if event.state == ElementState::Pressed {
                match event.logical_key {
                    Key::Named(NamedKey::Escape) => {
                        app.running = false;
                    }
                    Key::Named(NamedKey::Backspace) => {
                        app.event_bus.push(UiEvent::Backspace);
                    }
                    Key::Named(NamedKey::Enter) => {
                        app.event_bus.push(UiEvent::TextInput("\n".to_string()));
                    }
                    Key::Named(NamedKey::Tab) => {
                        app.event_bus.push(UiEvent::TextInput("\t".to_string()));
                    }
                    _ => {
                        if let Some(ref text) = event.text
                            && !text.is_empty()
                            && !text.chars().any(char::is_control)
                        {
                            app.event_bus.push(UiEvent::TextInput(text.to_string()));
                        }
                    }
                }
            }
        }
        WindowEvent::CursorMoved { position, .. } => {
            app.last_mouse_pos = Point::new(position.x as f32, position.y as f32);
            input.mouse_moved = true;
        }
        WindowEvent::MouseInput { state, button, .. } => {
            if *button == MouseButton::Left {
                if *state == ElementState::Pressed {
                    input.clicked_pos = Some(app.last_mouse_pos);
                } else {
                    input.mouse_released = true;
                }
            }
        }
        WindowEvent::CursorLeft { .. } => {
            let prev_hovered = app.hovered_btn;
            app.hovered_btn = None;
            if let Some(prev) = prev_hovered {
                app.event_bus.push(UiEvent::HoverEnd(prev));
            }
            app.last_mouse_pos = Point::new(-9999.0, -9999.0);
            input.mouse_moved = true;
        }
        WindowEvent::Resized(size) => {
            let width = f32::from(u16::try_from(size.width).unwrap_or(0));
            let height = f32::from(u16::try_from(size.height).unwrap_or(0));
            app.event_bus.push(UiEvent::WindowResized(width, height));
        }
        WindowEvent::MouseWheel { delta, .. } => match delta {
            MouseScrollDelta::LineDelta(x, y) => {
                input.scroll_events.push((*x as i32, *y as i32));
            }
            MouseScrollDelta::PixelDelta(PhysicalPosition { x, y }) => {
                input.scroll_events.push((*x as i32, *y as i32));
            }
        },
        _ => {}
    }
}
