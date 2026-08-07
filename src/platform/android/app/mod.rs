pub mod physics_sync;
pub mod state;

pub use physics_sync::ScrollPhysics;
pub use state::{AppState, KineticScroll};

use crate::core::render::draw::draw_ui;
use crate::core::style::BACKGROUND;
use crate::core::ui::event::UiEvent;
use crate::core::{calculate_layout, Renderer, ScreenMetrics, Size};
use crate::dev_err;
use crate::dev_log;
use android_activity::{AndroidApp, InputStatus, MainEvent, PollEvent};
use obfstr::obfstr;
use std::sync::Arc;
use std::time::Duration;

use super::window::EglContextState;

#[unsafe(no_mangle)]
#[allow(clippy::pedantic, clippy::too_many_lines)]
pub fn android_main(app: AndroidApp) {
    let mut egl_state: Option<EglContextState> = None;
    let mut state = AppState::new(&app);

    let mut root_element = crate::core::types::Element::Container {
        id: None,
        style: crate::core::style::Style::default(),
        children: vec![],
    };

    let mut last_frame_time = std::time::Instant::now();

    while state.running {
        let now = std::time::Instant::now();
        let dt = now.duration_since(last_frame_time).as_secs_f32();
        last_frame_time = now;

        let current_ui_version =
            crate::core::types::UI_VERSION.load(std::sync::atomic::Ordering::Relaxed);
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
            Duration::from_millis(16)
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
                        if egl_state.is_none() {
                            match EglContextState::new() {
                                Ok(s) => egl_state = Some(s),
                                Err(e) => dev_err!(
                                    "{}: {e}",
                                    obfstr!("[EGL Error] Failed to create EGL state")
                                ),
                            }
                        }
                        if let Some(ref mut egl) = egl_state
                            && let Some(window) = app.native_window()
                        {
                            if let Err(e) = egl.bind_window(&window) {
                                dev_err!("{}: {e}", obfstr!("[EGL Error] Failed to bind window"));
                            } else {
                                dev_log!("{}", obfstr!("[EGL] Window successfully bound to EGL"));
                            }
                            let width = f32::from(u16::try_from(window.width()).unwrap_or(0));
                            let height = f32::from(u16::try_from(window.height()).unwrap_or(0));
                            state.event_bus.push(UiEvent::WindowResized(width, height));
                        }
                        state.cached_layout = None;
                        state.cached_max_scroll.clear();
                        state
                            .virtual_page_manager
                            .invalidate_on_resize(&mut root_element);
                        needs_redraw = true;
                    }
                    MainEvent::WindowResized { .. }
                    | MainEvent::ContentRectChanged { .. }
                    | MainEvent::RedrawNeeded { .. } => {
                        if let Some(ref mut egl) = egl_state
                            && let Some(window) = app.native_window()
                        {
                            let _ = egl.bind_window(&window);
                            let width = f32::from(u16::try_from(window.width()).unwrap_or(0));
                            let height = f32::from(u16::try_from(window.height()).unwrap_or(0));
                            state.event_bus.push(UiEvent::WindowResized(width, height));
                        }
                        state.cached_layout = None;
                        state.cached_max_scroll.clear();
                        state
                            .virtual_page_manager
                            .invalidate_on_resize(&mut root_element);
                        needs_redraw = true;
                    }
                    MainEvent::InputAvailable => {
                        if let Ok(mut iter) = app.input_events_iter() {
                            loop {
                                let had_event = iter.next(|input_event| {
                                    let layout_clone = state.cached_layout.clone();
                                    if let Some(ref cached) = layout_clone {
                                        super::input::handle_input_event(
                                            input_event,
                                            &mut state,
                                            &root_element,
                                            cached,
                                        )
                                    } else {
                                        InputStatus::Unhandled
                                    }
                                });

                                if !had_event {
                                    break;
                                }
                            }
                        }
                        needs_redraw = true;
                    }
                    MainEvent::TerminateWindow { .. } => {
                        if let Some(ref mut egl) = egl_state {
                            egl.unbind();
                        }
                    }
                    MainEvent::LowMemory => {
                        state.memory_pressure_pending = true;
                        needs_redraw = true;
                    }
                    MainEvent::Destroy => {
                        egl_state = None;
                        state.running = false;
                    }
                    _ => {}
                },
                _ => {}
            }
        });

        if needs_redraw
            && let Some(ref mut egl) = egl_state
            && let Some(ref mut renderer) = egl.renderer
            && let Some(window) = app.native_window()
        {
            if state.memory_pressure_pending {
                state.trim_memory(renderer);
            }

            state.plugin_registry.tick();

            let width =
                f32::from(u16::try_from(window.width()).expect("window width fits in u16"));
            let height =
                f32::from(u16::try_from(window.height()).expect("window height fits in u16"));

            let screen_changed = (width - state.cached_screen_size.0).abs() > 0.5
                || (height - state.cached_screen_size.1).abs() > 0.5;

            if screen_changed {
                state.cached_screen_size = (width, height);
                state.cached_safe_area = crate::platform::android::jni::get_safe_area(&app)
                    .map(|(top, bottom)| {
                        (
                            f32::from(i16::try_from(top).expect("safe-area top fits in i16")),
                            f32::from(
                                i16::try_from(bottom).expect("safe-area bottom fits in i16"),
                            ),
                        )
                    })
                    .unwrap_or((0.0, 0.0));
                state.cached_density = crate::platform::android::jni::get_density();

                state.cached_layout = None;
                state.cached_max_scroll.clear();

                let content_h = height - state.cached_safe_area.0 - state.cached_safe_area.1;
                crate::core::types::SCREEN_WIDTH
                    .store(width.to_bits(), std::sync::atomic::Ordering::Relaxed);
                crate::core::types::SCREEN_HEIGHT
                    .store(content_h.to_bits(), std::sync::atomic::Ordering::Relaxed);

                ui_changed = true;
            } else {
                let content_h = height - state.cached_safe_area.0 - state.cached_safe_area.1;
                crate::core::types::SCREEN_WIDTH
                    .store(width.to_bits(), std::sync::atomic::Ordering::Relaxed);
                crate::core::types::SCREEN_HEIGHT
                    .store(content_h.to_bits(), std::sync::atomic::Ordering::Relaxed);
            }

            let is_first_frame = matches!(&root_element, crate::core::types::Element::Container { children, .. } if children.is_empty());
            if ui_changed || is_first_frame {
                if let Some(new_root) = state.plugin_registry.build_ui() {
                    root_element = new_root;
                }
            }

            if screen_changed || is_first_frame {
                let content_h = height - state.cached_safe_area.0 - state.cached_safe_area.1;
                state
                    .virtual_page_manager
                    .on_viewport_resized(width, content_h, &mut root_element);

                if let Some((pixels, w, h)) =
                    crate::platform::android::jni::bridge::get_system_wallpaper_pixels(
                        width as u32,
                        height as u32,
                    )
                {
                    renderer.load_wallpaper(&pixels, w, h);
                }
            }

            state
                .virtual_page_manager
                .virtualize_tree(&mut root_element);

            physics_sync::sync_scroll_physics_from_tree(
                &root_element,
                &mut state.scroll_physics,
            );

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
                    state
                        .virtual_page_manager
                        .invalidate_on_resize(&mut root_element);
                }
            }

            let layout_tree = if !state.layout_dirty
                && let Some(ref cached) = state.cached_layout
            {
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
                target_id
                    .and_then(|id| state.cached_max_scroll.get(&id).copied())
                    .unwrap_or(0.0)
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
                    if phys.tick(dt) {
                        #[allow(unused_assignments)]
                        {
                            needs_redraw = true;
                        }
                    }

                    if let Some(snap_width) = phys.snap_x {
                        if snap_width > 0.0 {
                            let max_page = (phys.page_count.unwrap_or(1) as i32 - 1).max(0);
                            let max_x = snap_width * (max_page as f32);

                            phys.pos_x = phys.pos_x.clamp(0.0, max_x);

                            let current_page = (phys.pos_x / snap_width).round() as i32;
                            let clamped_page = current_page.clamp(0, max_page) as usize;

                            let page_window_changed =
                                state.virtual_page_manager.update_predicted_page(
                                    clamped_page,
                                    &mut root_element,
                                    phys.vel_x,
                                );

                            if page_window_changed {
                                state.cached_layout = None;
                                #[allow(unused_assignments)]
                                {
                                    needs_redraw = true;
                                }
                            }

                            phys.last_snap_page = clamped_page as i32;

                            if phys.snap_just_completed && phys.on_snap.is_some() {
                                state.event_bus.push(UiEvent::PageSnapped {
                                    widget_id: sv_id,
                                    page: clamped_page as i32,
                                });
                            }
                        }
                    }

                    let pos_x = phys.pos_x;
                    let pos_y = phys.pos_y;
                    physics_sync::inject_physics_to_tree(&mut root_element, sv_id, pos_x, pos_y);

                    physics_sync::update_indicator_dots_in_element(
                        &mut root_element,
                        phys.last_snap_page,
                        vmin_px,
                    );
                }
            }

            state.plugin_registry.dispatch(
                &state.event_bus,
                &state.style_map,
                &state.data_map,
                &state.action_queue,
            );

            if let Ok(mut q) = state.action_queue.lock() {
                for action in q.drain(..) {
                    match action {
                        crate::core::Action::LoadImage { id, src } => {
                            if let Some(pkg_name) = src.strip_prefix(obfstr!("app-icon://")) {
                                if let Some((pixels, w, h)) =
                                    crate::platform::android::jni::bridge::get_app_icon_pixels(
                                        pkg_name,
                                    )
                                {
                                    renderer.load_image(&id, &pixels, w, h);
                                    dev_log!(
                                        "{} {}",
                                        obfstr!("Successfully loaded app icon for"),
                                        pkg_name
                                    );
                                } else {
                                    dev_err!(
                                        "{} {}",
                                        obfstr!("Failed to load app icon for"),
                                        pkg_name
                                    );
                                }
                            } else {
                                let asset_path =
                                    src.replace(obfstr!(".plugins/"), "").replace('\\', "/");

                                if let Ok(cstr) = std::ffi::CString::new(asset_path.clone()) {
                                    if let Some(mut asset) =
                                        app.asset_manager().open(cstr.as_c_str())
                                    {
                                        use std::io::Read;
                                        let mut buffer = Vec::new();
                                        if asset.read_to_end(&mut buffer).is_ok() {
                                            if let Ok(img) = image::load_from_memory(&buffer) {
                                                let rgba = img.to_rgba8();
                                                let (w, h) = rgba.dimensions();
                                                renderer.load_image(&id, rgba.as_raw(), w, h);
                                                dev_log!(
                                                    "{} {} {}",
                                                    obfstr!("Successfully loaded image"),
                                                    asset_path,
                                                    obfstr!("from Android assets")
                                                );
                                            } else {
                                                dev_err!(
                                                    "{} {}",
                                                    obfstr!("Failed to parse image data for"),
                                                    asset_path
                                                );
                                            }
                                        }
                                    } else {
                                        dev_err!(
                                            "{} {}",
                                            obfstr!("Failed to open image asset:"),
                                            asset_path
                                        );
                                    }
                                }
                            }
                        }
                        crate::core::Action::FocusTextInput(_id) => {
                            app.show_soft_input(true);
                        }
                        crate::core::Action::BlurTextInput => {
                            app.hide_soft_input(true);
                        }
                        _ => {
                            let _ = crate::platform::android::jni::intent::launch_action(action);
                        }
                    }
                }
            }

            renderer.begin_frame(width, height);
            renderer.clear(BACKGROUND);
            renderer.draw_wallpaper(width, height);

            draw_ui(
                renderer,
                &root_element,
                &layout_tree,
                &metrics,
                &state.style_map,
                &state.data_map,
                &state.transition_manager,
                1.0,
                0.0,
                0.0,
            );

            renderer.end_frame();
            egl.swap_buffers();
        }
    }
}
