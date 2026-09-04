use super::helpers::collect_layout_rects;
use super::input::process_frame_inputs;
use super::state::{AppState, FrameInputState};
use crate::window::DesktopWindow;
use obfstr::obfstr;
use sniffer_core::dev_log;
use sniffer_core::ui::event::UiEvent;
use sniffer_core::{Action, Renderer, ScreenMetrics, Size, calculate_layout};
use sniffer_render::GlowRenderer;
use sniffer_render::draw::draw_ui;
use std::collections::HashMap;

const DESKTOP_BG_COLOR: u32 = 0x000B_0B0E;

#[allow(
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss
)]
pub fn render_desktop_frame(
    app: &mut AppState,
    input: &mut FrameInputState,
    desktop: &DesktopWindow,
    renderer: &mut GlowRenderer,
    last_frame_time: &mut std::time::Instant,
) {
    let now = std::time::Instant::now();
    #[cfg(feature = "devkit")]
    let render_start = now;
    let dt = now.duration_since(*last_frame_time).as_secs_f32();
    *last_frame_time = now;

    let (log_w, log_h) = desktop.size();
    let (phys_w, phys_h) = desktop.drawable_size();

    let width = f32::from(u16::try_from(phys_w).unwrap_or(0));
    let height = f32::from(u16::try_from(phys_h).unwrap_or(0));

    sniffer_core::types::SCREEN_WIDTH.store(width.to_bits(), std::sync::atomic::Ordering::Relaxed);
    sniffer_core::types::SCREEN_HEIGHT
        .store(height.to_bits(), std::sync::atomic::Ordering::Relaxed);

    app.plugin_registry.tick();
    if let Ok(mut reg) = app.pkg_registry.write() {
        reg.tick_all(dt);
    }

    let mut root_element =
        app.plugin_registry
            .build_ui()
            .unwrap_or_else(|| sniffer_core::types::Element::Container {
                id: None,
                style: sniffer_core::style::Style::default(),
                children: vec![],
            });

    sniffer_core::physics::sync_scroll_physics_from_tree(&root_element, &mut app.scroll_physics);

    let vmin_px = width.min(height) / 100.0;
    let phys_ids: Vec<u64> = app.scroll_physics.keys().copied().collect();
    for sv_id in phys_ids {
        if let Some(phys) = app.scroll_physics.get_mut(&sv_id) {
            let _ = phys.tick(dt);

            if let Some(snap_width) = phys.snap_x {
                if snap_width > 0.0 {
                    let max_page = (phys.page_count.unwrap_or(1) as i32 - 1).max(0);
                    let current_page = (phys.pos_x / snap_width).round() as i32;
                    let clamped_page = current_page.clamp(0, max_page) as usize;

                    phys.last_snap_page = clamped_page as i32;

                    if phys.snap_just_completed && phys.on_snap.is_some() {
                        app.event_bus.push(UiEvent::PageSnapped {
                            widget_id: sv_id,
                            page: clamped_page as i32,
                        });
                    }

                    let _ = sniffer_core::physics::update_indicator_dots_in_element(
                        &mut root_element,
                        phys.last_snap_page,
                        vmin_px,
                    );
                }
            }

            sniffer_core::physics::inject_physics_to_tree(
                &mut root_element,
                sv_id,
                phys.pos_x,
                phys.pos_y,
            );
        }
    }

    app.transition_manager.sync_tree(&root_element);
    let _ = app.transition_manager.tick(dt);

    let scale_x = width / f32::from(u16::try_from(log_w).unwrap_or(1));
    let scale_y = height / f32::from(u16::try_from(log_h).unwrap_or(1));

    let mut scaled_last_mouse_pos = app.last_mouse_pos;
    if (scaled_last_mouse_pos.x - -9999.0).abs() > f32::EPSILON {
        scaled_last_mouse_pos.x *= scale_x;
        scaled_last_mouse_pos.y *= scale_y;
    }

    let scaled_clicked_pos = input.clicked_pos.map(|mut pos| {
        pos.x *= scale_x;
        pos.y *= scale_y;
        pos
    });

    let metrics = ScreenMetrics::from_dpi(width, height, desktop.dpi);
    let layout_tree = calculate_layout(&root_element, Size::new(width, height), 0.0, 0.0);

    process_frame_inputs(
        app,
        input,
        &root_element,
        &layout_tree,
        scaled_last_mouse_pos,
        scaled_clicked_pos,
        scale_x,
        scale_y,
    );

    app.plugin_registry.dispatch(
        &app.event_bus,
        &app.style_map,
        &app.data_map,
        &app.action_queue,
    );

    if let Ok(mut q) = app.action_queue.lock() {
        for action in q.drain(..) {
            match action {
                Action::LoadImage { id, src } => {
                    if src.starts_with(obfstr!("app-icon://")) {
                        let w = 64;
                        let h = 64;
                        let pixels = [0, 255, 0, 255].repeat((w * h) as usize);
                        renderer.load_image(&id, &pixels, w, h);
                        renderer.load_image(&src, &pixels, w, h);
                        dev_log!("{} {src}", obfstr!("Loaded dummy app icon for"));
                    }
                }
                Action::LaunchApp { package_name, .. } => {
                    dev_log!(
                        "{} {package_name}",
                        obfstr!("Desktop Preview: Launch App triggered")
                    );
                }
                _ => {}
            }
        }
    }

    renderer.begin_frame(width, height);
    renderer.clear(DESKTOP_BG_COLOR);

    let _rendered_nodes = draw_ui(
        renderer,
        &root_element,
        &layout_tree,
        &metrics,
        &app.style_map,
        &app.data_map,
        &app.transition_manager,
        1.0,
        0.0,
        0.0,
    );

    let mut layout_rects = HashMap::new();
    collect_layout_rects(&layout_tree, &mut layout_rects);
    if let Ok(mut reg) = app.pkg_registry.write() {
        reg.render_widgets(renderer, &layout_rects, None);
    }

    #[cfg(feature = "devkit")]
    {
        if let Ok(prof) = app.profiler.lock() {
            sniffer_core::profiler::render_devkit_debug_overlays(
                renderer,
                &prof,
                &root_element,
                &layout_tree,
            );
        }
    }

    renderer.end_frame();
    #[cfg(feature = "devkit")]
    let draw_end = std::time::Instant::now();

    let _ = desktop.swap_buffers();
    #[cfg(feature = "devkit")]
    let swap_end = std::time::Instant::now();

    #[cfg(feature = "devkit")]
    if let Ok(mut prof) = app.profiler.lock() {
        prof.record_frame(render_start, render_start, draw_end, swap_end);
        prof.touch_telemetry.active_pointers = usize::from(app.is_mouse_down);
        prof.touch_telemetry.touch_x = scaled_last_mouse_pos.x;
        prof.touch_telemetry.touch_y = scaled_last_mouse_pos.y;
        prof.touch_telemetry.target_element = app
            .hovered_btn
            .map_or_else(|| "None".to_string(), |b| format!("btn_{b:x}"));
        prof.touch_telemetry.gesture = if app.is_mouse_down {
            "DRAG / MOUSE".to_string()
        } else {
            "HOVER".to_string()
        };
    }

    *input = FrameInputState::default();
}
