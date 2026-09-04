use super::frame::render_desktop_frame;
use super::state::{AppState, FrameInputState};
use crate::window::DesktopWindow;
use sniffer_core::ui::event::UiEvent;
use sniffer_render::GlowRenderer;

#[allow(
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::needless_pass_by_value,
    clippy::cast_possible_truncation
)]
pub fn run_loop(
    mut app: AppState,
    event_loop: winit::event_loop::EventLoop<()>,
    desktop: DesktopWindow,
    mut renderer: GlowRenderer,
) -> Result<(), String> {
    let mut last_frame_time = std::time::Instant::now();
    let mut input = FrameInputState::default();

    let (phys_w, phys_h) = desktop.drawable_size();
    let width = f32::from(u16::try_from(phys_w).unwrap_or(0));
    let height = f32::from(u16::try_from(phys_h).unwrap_or(0));
    sniffer_core::types::SCREEN_WIDTH.store(width.to_bits(), std::sync::atomic::Ordering::Relaxed);
    sniffer_core::types::SCREEN_HEIGHT
        .store(height.to_bits(), std::sync::atomic::Ordering::Relaxed);

    let smoke_test_target_frames = std::env::var("SNIFFER_SMOKE_TEST_FRAMES")
        .ok()
        .and_then(|v| v.parse::<u32>().ok());
    let screenshot_path = std::env::var("SNIFFER_SCREENSHOT_PATH").ok();
    let mut rendered_frames: u32 = 0;

    desktop.window.request_redraw();

    #[allow(deprecated)]
    event_loop
        .run(move |event, active_event_loop| match event {
            winit::event::Event::WindowEvent { event, .. } => {
                match &event {
                    winit::event::WindowEvent::CloseRequested => {
                        if let Ok(mut reg) = app.pkg_registry.write() {
                            reg.unload_all();
                        }
                        app.running = false;
                        active_event_loop.exit();
                        return;
                    }
                    winit::event::WindowEvent::Resized(size) => {
                        desktop.resize_surface(size.width, size.height);
                        let (phys_w, phys_h) = desktop.drawable_size();
                        let width = f32::from(u16::try_from(phys_w).unwrap_or(0));
                        let height = f32::from(u16::try_from(phys_h).unwrap_or(0));
                        sniffer_core::types::SCREEN_WIDTH
                            .store(width.to_bits(), std::sync::atomic::Ordering::Relaxed);
                        sniffer_core::types::SCREEN_HEIGHT
                            .store(height.to_bits(), std::sync::atomic::Ordering::Relaxed);

                        app.event_bus.push(UiEvent::WindowResized(width, height));
                        desktop.window.request_redraw();
                    }
                    winit::event::WindowEvent::ScaleFactorChanged { .. } => {
                        desktop.window.request_redraw();
                    }
                    winit::event::WindowEvent::RedrawRequested => {
                        render_desktop_frame(
                            &mut app,
                            &mut input,
                            &desktop,
                            &mut renderer,
                            &mut last_frame_time,
                        );

                        if let Some(target) = smoke_test_target_frames {
                            rendered_frames = rendered_frames.saturating_add(1);
                            if rendered_frames >= target {
                                if let Some(ref path) = screenshot_path {
                                    let (w, h) = desktop.drawable_size();
                                    if let Err(e) = renderer.capture_framebuffer_png(w, h, path) {
                                        eprintln!("Failed to save smoke test screenshot: {e}");
                                    } else {
                                        println!("Smoke test screenshot saved to: {path}");
                                    }
                                }
                                if let Ok(mut reg) = app.pkg_registry.write() {
                                    reg.unload_all();
                                }
                                app.running = false;
                                active_event_loop.exit();
                                return;
                            }
                        }
                    }
                    _ => {}
                }

                crate::input::handle_winit_event(&event, &mut app, &mut input);
                desktop.window.request_redraw();
            }
            winit::event::Event::AboutToWait => {
                if !app.running {
                    active_event_loop.exit();
                    return;
                }

                desktop.window.request_redraw();
                active_event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
            }
            _ => {}
        })
        .map_err(|e| e.to_string())?;

    Ok(())
}
