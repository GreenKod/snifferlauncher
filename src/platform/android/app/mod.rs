pub mod physics_sync;
pub mod state;

pub use physics_sync::ScrollPhysics;
pub use state::{AppState, KineticScroll};

use crate::core::render::draw::draw_ui;
use crate::core::ui::event::UiEvent;
use crate::core::{calculate_layout, Renderer, ScreenMetrics, Size};
use crate::dev_err;
use crate::dev_log;
use android_activity::{AndroidApp, MainEvent, PollEvent};
use obfstr::obfstr;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use glow::HasContext;

use super::window::EglContextState;

pub enum RenderMessage {
    InitWindow(usize, f32, f32),
    WindowResized(usize, f32, f32),
    TerminateWindow,
    LowMemory,
    Destroy,
}

#[derive(Clone)]
pub struct SharedRenderState {
    pub root_element: crate::core::types::Element,
    pub layout_tree: Arc<crate::core::layout::LayoutNode>,
    pub metrics: ScreenMetrics,
    pub style_map: crate::core::ui::style_map::StyleMap,
    pub data_map: crate::core::ui::data_map::DataMap,
    pub transition_manager: crate::core::anim::TransitionManager,
    pub shaders_warmed_up: bool,
}

#[unsafe(no_mangle)]
#[allow(clippy::pedantic, clippy::too_many_lines)]
pub fn android_main(app: AndroidApp) {
    crate::platform::android::jni::bridge::init_app_list_cache();
    crate::platform::android::jni::bridge::init_icon_worker_pool();
    crate::platform::android::jni::bridge::set_show_wallpaper_flag(&app);

    let mut state = AppState::new(&app);

    let (initial_width, initial_height) = app.native_window().map_or((1080.0, 1920.0), |window| {
        let w = f32::from(u16::try_from(window.width()).unwrap_or(1080));
        let h = f32::from(u16::try_from(window.height()).unwrap_or(1920));
        if w > 0.0 && h > 0.0 { (w, h) } else { (1080.0, 1920.0) }
    });

    let initial_root = crate::core::types::Element::Container {
        id: None,
        style: crate::core::style::Style::default(),
        children: vec![],
    };
    let initial_layout = Arc::new(crate::core::layout::LayoutNode::new(initial_root.clone(), crate::core::Rect { x: 0.0, y: 0.0, width: initial_width, height: initial_height }));

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

    let render_app_clone = app.clone();
    let render_state_clone = shared_render_state.clone();
    let action_queue_clone = state.action_queue.clone();
    let profiler_clone = state.profiler.clone();

    std::thread::spawn(move || {
        let mut egl_state: Option<EglContextState> = None;
        let mut is_window_bound = false;

        loop {
            while let Ok(msg) = render_rx.try_recv() {
                match msg {
                    RenderMessage::InitWindow(ptr, _w, _h) => {
                        if egl_state.is_none() {
                            match EglContextState::new() {
                                Ok(s) => egl_state = Some(s),
                                Err(e) => dev_err!("{}: {e}", obfstr!("[EGL Error] Failed to create EGL state")),
                            }
                        }
                        if let Some(ref mut egl) = egl_state {
                            if let Err(e) = egl.bind_window(ptr as *mut std::ffi::c_void) {
                                dev_err!("{}: {e}", obfstr!("[EGL Error] Failed to bind window"));
                            } else {
                                dev_log!("{}", obfstr!("[EGL] Window successfully bound to EGL"));
                                is_window_bound = true;
                            }
                        }
                    }
                    RenderMessage::WindowResized(ptr, _w, _h) => {
                        if let Some(ref mut egl) = egl_state {
                            let _ = egl.bind_window(ptr as *mut std::ffi::c_void);
                            is_window_bound = true;
                        }
                    }
                    RenderMessage::TerminateWindow => {
                        if let Some(ref mut egl) = egl_state {
                            egl.unbind();
                            is_window_bound = false;
                        }
                    }
                    RenderMessage::LowMemory => {
                        if let Some(ref mut egl) = egl_state {
                            if let Some(ref mut renderer) = egl.renderer {
                                unsafe {
                                    renderer.texture_cache.textures.retain(|key, handle| {
                                        if key.as_str() == "__system_wallpaper__" {
                                            true
                                        } else {
                                            renderer.gl.delete_texture(handle.texture);
                                            false
                                        }
                                    });
                                    renderer.texture_cache.total_vram_bytes = 0;
                                }
                            }
                        }
                    }
                    RenderMessage::Destroy => {
                        return;
                    }
                }
            }

            if is_window_bound {
                if let Some(ref mut egl) = egl_state {
                    if let Some(ref mut renderer) = egl.renderer {
                        if let Ok(mut q) = action_queue_clone.lock() {
                            let mut unhandled = Vec::new();
                            let mut loaded_textures = 0;
                            
                            for action in q.drain(..) {
                                if loaded_textures >= 16 {
                                    unhandled.push(action);
                                    continue;
                                }
                                match action {
                                    crate::core::Action::LoadImage { id: _, src } => {
                                        if let Some(pkg_name) = src.strip_prefix(obfstr!("app-icon://")) {
                                            crate::platform::android::jni::bridge::request_async_app_icon(pkg_name);
                                        } else {
                                            let asset_path = format!("{}/{}", obfstr!(".plugins"), src);
                                            if let Ok(cstr) = std::ffi::CString::new(asset_path.clone()) {
                                                if let Some(mut asset) = render_app_clone.asset_manager().open(cstr.as_c_str()) {
                                                    use std::io::Read;
                                                    let mut buffer = Vec::new();
                                                    if asset.read_to_end(&mut buffer).is_ok() {
                                                        if let Ok(img) = image::load_from_memory(&buffer) {
                                                            let rgba = img.to_rgba8();
                                                            let (w, h) = rgba.dimensions();
                                                            renderer.load_image(&src, rgba.as_raw(), w, h);
                                                            loaded_textures += 1;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    _ => {
                                        unhandled.push(action);
                                    }
                                }
                            }
                            
                            if !unhandled.is_empty() {
                                *q = unhandled;
                            }
                        }

                        let render_start = std::time::Instant::now();
                        let current_state = render_state_clone.read().unwrap().clone();
                        
                        let width = current_state.metrics.physical_width;
                        let height = current_state.metrics.physical_height;


                        while let Some(res) = crate::platform::android::jni::bridge::poll_async_app_icon() {
                            let image_id = format!("{}{}", obfstr!("app-icon://"), res.package_name);
                            renderer.load_image(&image_id, &res.pixels, res.width, res.height);
                        }

                        renderer.begin_frame(width, height);
                        
                        // Perform Shader Pre-warm on the very first actual frame
                        if !current_state.shaders_warmed_up {
                            renderer.warm_up_shaders();
                            if let Ok(mut rs) = render_state_clone.write() {
                                let mut updated = (**rs).clone();
                                updated.shaders_warmed_up = true;
                                *rs = Arc::new(updated);
                            }
                        }

                        renderer.clear(0x00000000); // Fully transparent — OS compositor renders wallpaper behind

                        let rendered_nodes = draw_ui(
                            renderer,
                            &current_state.root_element,
                            &current_state.layout_tree,
                            &current_state.metrics,
                            &current_state.style_map,
                            &current_state.data_map,
                            &current_state.transition_manager,
                            1.0,
                            0.0,
                            0.0,
                        );

                        #[cfg(feature = "devkit")]
                        {
                            if let Ok(prof) = profiler_clone.lock() {
                                crate::core::profiler::render_devkit_hud(renderer, &prof, rendered_nodes, width);
                            }
                        }

                        renderer.end_frame();
                        let draw_end = std::time::Instant::now();

                        egl.swap_buffers();
                        let swap_end = std::time::Instant::now();

                        if let Ok(mut prof) = profiler_clone.lock() {
                            prof.record_frame(render_start, render_start, draw_end, swap_end);
                        }
                    }
                }
            } else {
                std::thread::sleep(Duration::from_millis(16));
            }
        }
    });

    let mut last_frame_time = std::time::Instant::now();

    while state.running {
        let frame_start = std::time::Instant::now();
        let dt = frame_start.duration_since(last_frame_time).as_secs_f32();
        last_frame_time = frame_start;

        state.plugin_registry.tick();

        let current_ui_version = crate::core::types::UI_VERSION.load(std::sync::atomic::Ordering::Relaxed);
        let mut ui_changed = current_ui_version != state.last_ui_version;

        let physics_active = state
            .scroll_physics
            .values()
            .any(|p| p.is_dragging || p.vel_x.abs() > 0.5 || p.snap_target_x.is_some());
            
        let mut needs_redraw = !state.kinetic_scrolls.is_empty()
            || state.transition_manager.states.values().any(|s| s.is_active)
            || physics_active
            || ui_changed;

        let poll_timeout = if needs_redraw {
            Duration::ZERO
        } else {
            Duration::from_millis(8)
        };

        app.poll_events(Some(poll_timeout), |event| {
            match event {
                PollEvent::Wake | PollEvent::Timeout => {
                    if !state.kinetic_scrolls.is_empty()
                        || state.transition_manager.states.values().any(|s| s.is_active)
                    {
                        needs_redraw = true;
                    }
                }
                PollEvent::Main(main_event) => match main_event {
                    MainEvent::InitWindow { .. } => {
                        if let Some(window) = app.native_window() {
                            let width = f32::from(u16::try_from(window.width()).unwrap_or(0));
                            let height = f32::from(u16::try_from(window.height()).unwrap_or(0));
                            let (top_sa, bot_sa) = crate::platform::android::jni::get_safe_area(&app)
                                .map(|(top, bottom)| {
                                    (
                                        f32::from(i16::try_from(top).unwrap_or(0)),
                                        f32::from(i16::try_from(bottom).unwrap_or(0)),
                                    )
                                })
                                .unwrap_or((0.0, 0.0));
                            let content_h = height - top_sa - bot_sa;
                            crate::core::types::SCREEN_WIDTH.store(width.to_bits(), std::sync::atomic::Ordering::Relaxed);
                            crate::core::types::SCREEN_HEIGHT.store(content_h.to_bits(), std::sync::atomic::Ordering::Relaxed);
                            state.event_bus.push(UiEvent::WindowResized(width, height));
                            
                            let native_window_ptr = window.ptr().as_ptr().cast::<std::ffi::c_void>() as usize;
                            let _ = render_tx.send(RenderMessage::InitWindow(native_window_ptr, width, height));
                        }
                        state.cached_layout = None;
                        state.cached_max_scroll.clear();
                        state.kinetic_scrolls.clear();
                        state.active_scrollview_drag = None;
                        state.virtual_page_manager.invalidate_on_resize(&mut root_element);
                        needs_redraw = true;
                    }
                    MainEvent::WindowResized { .. }
                    | MainEvent::ContentRectChanged { .. }
                    | MainEvent::RedrawNeeded { .. } => {
                        if let Some(window) = app.native_window() {
                            let width = f32::from(u16::try_from(window.width()).unwrap_or(0));
                            let height = f32::from(u16::try_from(window.height()).unwrap_or(0));
                            let (top_sa, bot_sa) = crate::platform::android::jni::get_safe_area(&app)
                                .map(|(top, bottom)| {
                                    (
                                        f32::from(i16::try_from(top).unwrap_or(0)),
                                        f32::from(i16::try_from(bottom).unwrap_or(0)),
                                    )
                                })
                                .unwrap_or((0.0, 0.0));
                            let content_h = height - top_sa - bot_sa;
                            crate::core::types::SCREEN_WIDTH.store(width.to_bits(), std::sync::atomic::Ordering::Relaxed);
                            crate::core::types::SCREEN_HEIGHT.store(content_h.to_bits(), std::sync::atomic::Ordering::Relaxed);
                            state.event_bus.push(UiEvent::WindowResized(width, height));
                            
                            let native_window_ptr = window.ptr().as_ptr().cast::<std::ffi::c_void>() as usize;
                            let _ = render_tx.send(RenderMessage::WindowResized(native_window_ptr, width, height));
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
                        state.virtual_page_manager.invalidate_on_resize(&mut root_element);
                        needs_redraw = true;
                    }
                    MainEvent::InputAvailable => {
                        if let Ok(mut iter) = app.input_events_iter() {
                            loop {
                                let had_event = iter.next(|input_event| {
                                    let layout_clone = if let Some(ref cached) = state.cached_layout {
                                        cached.clone()
                                    } else {
                                        let (w, h) = app.native_window().map_or((1080.0, 1920.0), |win| (f32::from(u16::try_from(win.width()).unwrap_or(1080)), f32::from(u16::try_from(win.height()).unwrap_or(1920))));
                                        Arc::new(crate::core::layout::LayoutNode::new(root_element.clone(), crate::core::Rect { x: 0.0, y: 0.0, width: w, height: h })) 
                                    };
                                    super::input::handle_input_event(
                                        input_event,
                                        &mut state,
                                        &root_element,
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
            }
        });

        if needs_redraw {
            let mut width = state.cached_screen_size.0;
            let mut height = state.cached_screen_size.1;
            
            if let Some(window) = app.native_window() {
                width = f32::from(u16::try_from(window.width()).expect("window width fits in u16"));
                height = f32::from(u16::try_from(window.height()).expect("window height fits in u16"));
            }

            if width < 1.0 || height < 1.0 {
                continue;
            }

            let screen_changed = (width - state.cached_screen_size.0).abs() > 0.5
                || (height - state.cached_screen_size.1).abs() > 0.5;

            if screen_changed {
                state.cached_screen_size = (width, height);
                state.cached_safe_area = crate::platform::android::jni::get_safe_area(&app)
                    .map(|(top, bottom)| {
                        (
                            f32::from(i16::try_from(top).expect("safe-area top fits in i16")),
                            f32::from(i16::try_from(bottom).expect("safe-area bottom fits in i16")),
                        )
                    })
                    .unwrap_or((0.0, 0.0));
                state.cached_density = crate::platform::android::jni::get_density();

                state.cached_layout = None;
                state.cached_max_scroll.clear();

                let content_h = height - state.cached_safe_area.0 - state.cached_safe_area.1;
                crate::core::types::SCREEN_WIDTH.store(width.to_bits(), std::sync::atomic::Ordering::Relaxed);
                crate::core::types::SCREEN_HEIGHT.store(content_h.to_bits(), std::sync::atomic::Ordering::Relaxed);

                ui_changed = true;
            }

            let is_first_frame = matches!(&root_element, crate::core::types::Element::Container { children, .. } if children.is_empty());
            if ui_changed || is_first_frame {
                if let Some(new_root) = state.plugin_registry.build_ui() {
                    root_element = new_root;
                    state.layout_dirty = true;
                }
            }

            if screen_changed || is_first_frame {
                let content_h = height - state.cached_safe_area.0 - state.cached_safe_area.1;
                state.virtual_page_manager.on_viewport_resized(width, content_h, &mut root_element);
            }

            state.virtual_page_manager.virtualize_tree(&mut root_element);
            physics_sync::sync_scroll_physics_from_tree(&root_element, &mut state.scroll_physics);
            state.transition_manager.sync_tree(&root_element);
            let _ = state.transition_manager.tick(dt);

            let (safe_area_top, safe_area_bottom) = state.cached_safe_area;
            let (density, scaled_density) = state.cached_density;

            let metrics = ScreenMetrics::from_scale(
                width,
                height - safe_area_top - safe_area_bottom,
                density,
                scaled_density,
            );

            if let Some(ref cached) = state.cached_layout {
                let cached_is_landscape = cached.rect.width > cached.rect.height;
                let window_is_landscape = width > height;
                if cached_is_landscape != window_is_landscape {
                    state.cached_layout = None;
                    state.layout_dirty = true;
                    state.virtual_page_manager.invalidate_on_resize(&mut root_element);
                }
            }

            let layout_tree = if !state.layout_dirty && let Some(ref cached) = state.cached_layout {
                cached.clone()
            } else {
                state.last_ui_version = current_ui_version;
                let fresh = Arc::new(calculate_layout(
                    &root_element,
                    Size::new(width, height - safe_area_top - safe_area_bottom),
                    0.0,
                    safe_area_top,
                ));
                state::update_max_scroll_cache(&mut state, &root_element, &fresh);
                state.cached_layout = Some(fresh.clone());
                state.layout_dirty = false;
                fresh
            };

            let get_max_scroll = |target_id: Option<u64>| -> f32 {
                target_id.and_then(|id| state.cached_max_scroll.get(&id).copied()).unwrap_or(0.0)
            };

            state.kinetic_scrolls.retain_mut(|k| {
                if k.velocity_x.abs() > 0.5 || k.velocity_y.abs() > 0.5 {
                    state.event_bus.push(UiEvent::Scroll(
                        Some(k.sv_id),
                        k.velocity_x,
                        k.velocity_y,
                        999_999.0,
                        get_max_scroll(Some(k.sv_id)),
                    ));
                    k.velocity_x *= 0.88;
                    k.velocity_y *= 0.88;
                    true
                } else {
                    false
                }
            });

            let content_h_for_js = height - safe_area_top - safe_area_bottom;
            let vmin_px = width.min(content_h_for_js) / 100.0;
            let phys_ids: Vec<u64> = state.scroll_physics.keys().copied().collect();
            for sv_id in phys_ids {
                if let Some(phys) = state.scroll_physics.get_mut(&sv_id) {
                    let _ = phys.tick(dt);

                    if let Some(snap_width) = phys.snap_x {
                        if snap_width > 0.0 {
                            let max_page = (phys.page_count.unwrap_or(1) as i32 - 1).max(0);
                            let current_page = (phys.pos_x / snap_width).round() as i32;
                            let clamped_page = current_page.clamp(0, max_page) as usize;

                            let page_window_changed = state.virtual_page_manager.update_predicted_page(
                                clamped_page,
                                &mut root_element,
                                phys.vel_x,
                            );

                            if page_window_changed {
                                state.cached_layout = None;
                            }

                            phys.last_snap_page = clamped_page as i32;

                            if phys.snap_just_completed && phys.on_snap.is_some() {
                                state.event_bus.push(UiEvent::PageSnapped {
                                    widget_id: sv_id,
                                    page: clamped_page as i32,
                                });
                            }

                            let indicator_changed = physics_sync::update_indicator_dots_in_element(
                                &mut root_element,
                                phys.last_snap_page,
                                vmin_px,
                            );

                            if indicator_changed {
                                state.cached_layout = None;
                                state.layout_dirty = true;
                            }
                        }
                    }

                    let pos_x = phys.pos_x;
                    let pos_y = phys.pos_y;
                    physics_sync::inject_physics_to_tree(&mut root_element, sv_id, pos_x, pos_y);
                }
            }

            state.plugin_registry.tick();

            state.plugin_registry.dispatch(
                &state.event_bus,
                &state.style_map,
                &state.data_map,
                &state.action_queue,
            );

            if let Ok(mut q) = state.action_queue.lock() {
                let mut remaining = Vec::new();
                for action in q.drain(..) {
                    match action {
                        crate::core::Action::FocusTextInput(_id) => {
                            app.show_soft_input(true);
                        }
                        crate::core::Action::BlurTextInput => {
                            app.hide_soft_input(true);
                        }
                        crate::core::Action::LoadImage { id, src } => {
                            remaining.push(crate::core::Action::LoadImage { id, src });
                        }
                        _ => {
                            if let Err(e) = crate::platform::android::jni::intent::launch_action(action) {
                                crate::dev_err!("{}: {e}", obfstr!("Failed to launch action"));
                            }
                        }
                    }
                }
                *q = remaining;
            }

            let current_warmed_up = shared_render_state.read().unwrap().shaders_warmed_up;
            *shared_render_state.write().unwrap() = Arc::new(SharedRenderState {
                root_element: root_element.clone(),
                layout_tree,
                metrics,
                style_map: state.style_map.clone(),
                data_map: state.data_map.clone(),
                transition_manager: state.transition_manager.clone(),
                shaders_warmed_up: current_warmed_up,
            });
        }
    }
}
