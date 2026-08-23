use super::bridge::RenderMessage;
use super::state::AppState;
use android_activity::{AndroidApp, MainEvent, PollEvent};
use crossbeam_channel::Sender;
use sniffer_core::types::Element;
use sniffer_core::ui::event::UiEvent;
use std::sync::Arc;
use std::time::Duration;

#[allow(clippy::too_many_lines)]
pub fn poll_and_handle_events(
    app: &AndroidApp,
    state: &mut AppState,
    root_element: &mut Element,
    render_tx: &Sender<RenderMessage>,
) -> bool {
    let mut needs_redraw = false;

    app.poll_events(Some(Duration::from_millis(0)), |event| match event {
        PollEvent::Wake => {
            needs_redraw = true;
        }
        PollEvent::Main(main_event) => match main_event {
            MainEvent::Resume { .. } => {
                needs_redraw = true;
            }
            MainEvent::InitWindow { .. } => {
                if let Some(window) = app.native_window() {
                    let ptr = window.ptr().as_ptr() as usize;
                    let width = f32::from(u16::try_from(window.width()).unwrap_or(1080));
                    let height = f32::from(u16::try_from(window.height()).unwrap_or(1920));
                    let _ = render_tx.send(RenderMessage::InitWindow(ptr, width, height));
                }
                needs_redraw = true;
            }
            MainEvent::WindowResized { .. } | MainEvent::ConfigChanged { .. } => {
                let (width, height) = if let Some(window) = app.native_window() {
                    let ptr = window.ptr().as_ptr() as usize;
                    let w =
                        f32::from(u16::try_from(window.width()).expect("window width fits in u16"));
                    let h = f32::from(
                        u16::try_from(window.height()).expect("window height fits in u16"),
                    );
                    let _ = render_tx.send(RenderMessage::WindowResized(ptr, w, h));
                    (w, h)
                } else {
                    state.cached_screen_size
                };

                if width >= 1.0 && height >= 1.0 {
                    state.event_bus.push(UiEvent::WindowResized(width, height));
                }
                state.cached_layout = None;
                state.cached_max_scroll.clear();
                state.kinetic_scrolls.clear();
                state.active_scrollview_drag = None;
                for phys in state.scroll_physics.values_mut() {
                    phys.vel_x = 0.0;
                    phys.vel_y = 0.0;
                    phys.snap_target_x = None;
                }
                state
                    .virtual_page_manager
                    .invalidate_on_resize(root_element);
                needs_redraw = true;
            }
            MainEvent::InputAvailable => {
                if let Ok(mut iter) = app.input_events_iter() {
                    loop {
                        let had_event = iter.next(|input_event| {
                            let layout_clone = if let Some(ref cached) = state.cached_layout {
                                cached.clone()
                            } else {
                                // No layout yet — compute a best-effort one so hit-testing
                                // is correct from the very first touch event.
                                let (w, h) =
                                    app.native_window().map_or((1080.0_f32, 1920.0_f32), |win| {
                                        (
                                            f32::from(u16::try_from(win.width()).unwrap_or(1080)),
                                            f32::from(u16::try_from(win.height()).unwrap_or(1920)),
                                        )
                                    });
                                let (safe_top, safe_bottom) = state.cached_safe_area;
                                let content_h = (h - safe_top - safe_bottom).max(1.0);
                                Arc::new(sniffer_core::calculate_layout(
                                    root_element,
                                    sniffer_core::Size::new(w, content_h),
                                    0.0,
                                    safe_top,
                                ))
                            };
                            crate::input::handle_input_event(
                                input_event,
                                state,
                                root_element,
                                &layout_clone,
                            )
                        });

                        if !had_event {
                            break;
                        }
                    }
                }
                needs_redraw = true;
            }
            MainEvent::TerminateWindow { .. } => {
                let _ = render_tx.send(RenderMessage::TerminateWindow);
            }
            MainEvent::LowMemory => {
                let _ = render_tx.send(RenderMessage::LowMemory);
                state.cached_layout = None;
                state.layout_dirty = true;
            }
            MainEvent::Destroy => {
                let _ = render_tx.send(RenderMessage::Destroy);
                state.running = false;
            }
            _ => {}
        },
        _ => {}
    });

    needs_redraw
}
