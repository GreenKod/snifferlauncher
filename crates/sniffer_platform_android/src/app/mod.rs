pub mod actions;
pub mod bridge;
pub mod events;
pub mod frame;
pub mod physics_sync;
pub mod render_thread;
pub mod state;

pub use bridge::{AndroidHostBridge, RenderMessage, SharedRenderState};
pub use physics_sync::ScrollPhysics;
pub use state::{AppState, KineticScroll};

use android_activity::AndroidApp;
use sniffer_core::ScreenMetrics;
use std::sync::{Arc, RwLock};
use std::time::Duration;

#[allow(clippy::pedantic, clippy::too_many_lines)]
pub fn android_main(app: AndroidApp) {
    sniffer_render::draw::elements::set_icon_loader(crate::jni::bridge::request_async_app_icon);
    sniffer_plugin::js::host_bridge::set_host_bridge(Box::new(AndroidHostBridge));

    crate::jni::bridge::init_app_list_cache();
    crate::jni::bridge::init_icon_worker_pool();
    crate::jni::bridge::set_show_wallpaper_flag(&app);

    let mut state = AppState::new(&app);

    let (initial_width, initial_height) = app.native_window().map_or((1080.0, 1920.0), |window| {
        let w = f32::from(u16::try_from(window.width()).unwrap_or(1080));
        let h = f32::from(u16::try_from(window.height()).unwrap_or(1920));
        if w > 0.0 && h > 0.0 {
            (w, h)
        } else {
            (1080.0, 1920.0)
        }
    });

    let initial_root = sniffer_core::types::Element::Container {
        id: None,
        style: sniffer_core::style::Style::default(),
        children: vec![],
    };
    let initial_layout = Arc::new(sniffer_core::layout::LayoutNode::new(
        initial_root.clone(),
        sniffer_core::Rect {
            x: 0.0,
            y: 0.0,
            width: initial_width,
            height: initial_height,
        },
    ));

    let mut root_element = initial_root.clone();

    let shared_render_state = Arc::new(RwLock::new(Arc::new(SharedRenderState {
        root_element: initial_root,
        layout_tree: initial_layout,
        metrics: ScreenMetrics::default_mdpi(initial_width, initial_height),
        style_map: state.style_map.clone(),
        data_map: state.data_map.clone(),
        transition_manager: state.transition_manager.clone(),
        shaders_warmed_up: false,
    })));

    let (render_tx, render_rx) = crossbeam_channel::unbounded::<RenderMessage>();

    render_thread::spawn_render_thread(
        app.clone(),
        shared_render_state.clone(),
        state.action_queue.clone(),
        state.profiler.clone(),
        Arc::clone(&state.pkg_registry),
        render_rx,
    );

    let mut last_frame_time = std::time::Instant::now();

    while state.running {
        let frame_start = std::time::Instant::now();
        let dt = frame_start.duration_since(last_frame_time).as_secs_f32();
        last_frame_time = frame_start;

        state.plugin_registry.tick();
        if let Ok(mut reg) = state.pkg_registry.write() {
            reg.tick_all(dt);
        }

        if crate::jni::bridge::apps::take_app_list_updated() {
            if let Ok(apps) = crate::jni::get_application_list() {
                let _ = state
                    .plugin_registry
                    .vault()
                    .set_json("system.apps", &apps, "system");
                state
                    .plugin_registry
                    .broadcast("vault.changed:system.apps", "{}");
                state.plugin_registry.broadcast("system.apps", "{}");
                state.cached_layout = None;
                state.layout_dirty = true;
            }
        }

        let current_ui_version =
            sniffer_core::types::UI_VERSION.load(std::sync::atomic::Ordering::Relaxed);
        let ui_changed = state.layout_dirty || (current_ui_version != state.last_ui_version);

        let needs_redraw =
            events::poll_and_handle_events(&app, &mut state, &mut root_element, &render_tx);

        let has_active_physics = !state.kinetic_scrolls.is_empty()
            || state.scroll_physics.values().any(|p| {
                p.is_dragging
                    || p.snap_target_x.is_some()
                    || p.vel_x.abs() > 0.5
                    || p.vel_y.abs() > 0.5
            });

        let has_active_transitions = state.transition_manager.is_animating();
        let is_first_frame = matches!(
            &root_element,
            sniffer_core::types::Element::Container { children, .. } if children.is_empty()
        );

        let should_update = needs_redraw
            || ui_changed
            || has_active_physics
            || has_active_transitions
            || is_first_frame
            || state.cached_layout.is_none();

        if should_update {
            frame::update_and_render_state(
                &app,
                &mut state,
                &mut root_element,
                &shared_render_state,
                dt,
                ui_changed,
                current_ui_version,
            );
        }

        let elapsed = frame_start.elapsed();
        let target_frame_duration = if has_active_physics
            || has_active_transitions
            || state.active_scrollview_drag.is_some()
        {
            Duration::from_millis(8)
        } else {
            Duration::from_millis(16)
        };

        if elapsed < target_frame_duration {
            std::thread::sleep(target_frame_duration - elapsed);
        }
    }

    if let Ok(mut reg) = state.pkg_registry.write() {
        reg.unload_all();
    }
}
