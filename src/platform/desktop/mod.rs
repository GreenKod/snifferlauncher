use crate::core::render::draw::{draw_ui, find_clicked_button, find_hovered_button};
use crate::core::style::{BACKGROUND, WINDOW_HEIGHT, WINDOW_WIDTH};
use crate::core::ui::data_map::DataMap;
use crate::core::ui::event::{EventBus, UiEvent};
use crate::core::ui::style_map::StyleMap;
use crate::core::{Action, GlowRenderer, Point, Renderer, ScreenMetrics, Size, calculate_layout};
use crate::plugin::registry::PluginRegistry;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::Duration;

struct KineticScroll {
    sv_id: u64,
    velocity_x: f32,
    velocity_y: f32,
}

/// Run the desktop preview.
///
/// # Errors
///
/// Returns an error if SDL2 or GL initialization fails.
///
/// # Panics
///
/// Panics if mouse or drawable dimensions do not fit the narrow integer
/// conversions used for clippy-clean float handling.
#[allow(clippy::missing_panics_doc)]
#[allow(clippy::too_many_lines, clippy::similar_names)]
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize SDL2
    let sdl_context = sdl2::init()?;
    // Prefer X11 over Wayland/EGL for OpenGL compatibility
    sdl2::hint::set("SDL_VIDEO_DRIVER", "x11");
    let video_subsystem = sdl_context.video()?;

    let gl_attr = video_subsystem.gl_attr();
    gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
    gl_attr.set_context_version(3, 3);

    // 2. Create Window — dynamic size: 85% of display or fallback
    let display = video_subsystem.display_bounds(0).unwrap_or_else(|_| {
        sdl2::rect::Rect::new(
            0,
            0,
            u32::try_from(WINDOW_WIDTH).expect("window width fits in u32"),
            u32::try_from(WINDOW_HEIGHT).expect("window height fits in u32"),
        )
    });
    let init_width = display.width().saturating_mul(85) / 100;
    let init_height = display.height().saturating_mul(85) / 100;
    let init_width = if init_width < 400 { 400 } else { init_width };
    let init_height = if init_height < 600 { 600 } else { init_height };
    let window = video_subsystem
        .window("Platform-Agnostic Launcher", init_width, init_height)
        .opengl()
        .resizable()
        .position_centered()
        .build()?;

    // 3. Create GL context and wrap in Glow
    let _gl_context = window.gl_create_context()?;
    let gl = unsafe {
        glow::Context::from_loader_function(|s| video_subsystem.gl_get_proc_address(s).cast())
    };

    // 4. Read display DPI and build initial ScreenMetrics.
    //    SDL2 returns (diagonal_dpi, horizontal_dpi, vertical_dpi).
    //    We use horizontal DPI with 160 as the baseline (Android mdpi convention).
    let dpi = video_subsystem
        .display_dpi(0)
        .map_or(96.0, |(_ddpi, hdpi, _vdpi)| hdpi); // 96 dpi is a safe desktop fallback

    // Audiowide font embedded as binary
    let font_bytes: &[u8] = include_bytes!("../../fonts/audiowide.ttf");
    let mut renderer = unsafe { GlowRenderer::with_font(gl, Some(font_bytes))? };
    let mut event_pump = sdl_context.event_pump()?;

    // 6. Application logic state
    let mut last_mouse_pos = Point::new(-9999.0, -9999.0);
    let mut hovered_btn: Option<u64> = None;
    let mut active_scrollview_drag: Option<u64> = None;
    let mut last_drag_delta: (f32, f32) = (0.0, 0.0);

    let mut kinetic_scrolls: Vec<KineticScroll> = Vec::new();

    let event_bus = EventBus::default();
    let style_map = StyleMap::default();
    let data_map = DataMap::default();
    let action_queue = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

    let mut plugin_registry = PluginRegistry::default();

    // Register WASM plugins from assets/ui
    crate::plugin::PluginLoader::new("assets/ui")
        .register_all(&mut plugin_registry, action_queue.clone());

    let mut running = true;
    while running {
        // Trigger periodic tasks (like Garbage Collection) for plugins
        plugin_registry.tick();

        // Fetch dynamic UI tree from plugins EVERY FRAME
        let root_element =
            plugin_registry
                .build_ui()
                .unwrap_or_else(|| crate::core::types::Element::Container {
                    id: None,
                    style: crate::core::style::Style::default(),
                    children: vec![],
                });
        let mut clicked_pos = None;
        let mut mouse_released = false;
        let mut mouse_moved = false;
        let mut scroll_events = Vec::new();
        let mut drag_events = Vec::new();

        // 7. Poll SDL2 events
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    running = false;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Backspace),
                    ..
                } => {
                    event_bus.push(UiEvent::Backspace);
                }
                Event::MouseMotion {
                    x,
                    y,
                    xrel,
                    yrel,
                    mousestate,
                    ..
                } => {
                    last_mouse_pos = Point::new(
                        f32::from(i16::try_from(x).expect("mouse x fits in i16")),
                        f32::from(i16::try_from(y).expect("mouse y fits in i16")),
                    );
                    mouse_moved = true;
                    if mousestate.left() {
                        drag_events.push((-xrel, -yrel)); // Ters çeviriyoruz ki içerik fareye yapışsın
                    }
                }
                Event::MouseButtonDown {
                    mouse_btn: sdl2::mouse::MouseButton::Left,
                    x,
                    y,
                    ..
                } => {
                    clicked_pos = Some(Point::new(
                        f32::from(i16::try_from(x).expect("mouse x fits in i16")),
                        f32::from(i16::try_from(y).expect("mouse y fits in i16")),
                    ));
                }
                Event::MouseButtonUp {
                    mouse_btn: sdl2::mouse::MouseButton::Left,
                    ..
                } => {
                    mouse_released = true;
                }
                Event::Window {
                    win_event: sdl2::event::WindowEvent::Leave,
                    ..
                } => {
                    let prev_hovered = hovered_btn;
                    hovered_btn = None;
                    if let Some(prev) = prev_hovered {
                        event_bus.push(crate::core::ui::event::UiEvent::HoverEnd(prev));
                    }
                    // Move the mouse out of the layout entirely to prevent re-triggering hover
                    last_mouse_pos = crate::core::Point::new(-9999.0, -9999.0);
                    mouse_moved = true; // Force layout re-check to clear it naturally too if needed
                }
                Event::TextInput { text, .. } => {
                    event_bus.push(UiEvent::TextInput(text));
                }
                Event::MouseWheel { x, y, .. } => {
                    scroll_events.push((x, y));
                }
                _ => {}
            }
        }

        // 8. Query current drawable size each frame (handles resize and DPI changes)
        let (log_w, log_h) = window.size();
        let (phys_w, phys_h) = window.drawable_size();

        let width = f32::from(u16::try_from(phys_w).expect("drawable width fits in u16"));
        let height = f32::from(u16::try_from(phys_h).expect("drawable height fits in u16"));

        crate::core::types::SCREEN_WIDTH
            .store(width.to_bits(), std::sync::atomic::Ordering::Relaxed);
        crate::core::types::SCREEN_HEIGHT
            .store(height.to_bits(), std::sync::atomic::Ordering::Relaxed);

        let scale_x = width / f32::from(u16::try_from(log_w).unwrap_or(1));
        let scale_y = height / f32::from(u16::try_from(log_h).unwrap_or(1));

        // Scale mouse inputs to physical layout space
        let mut scaled_last_mouse_pos = last_mouse_pos;
        if (scaled_last_mouse_pos.x - -9999.0).abs() > f32::EPSILON {
            scaled_last_mouse_pos.x *= scale_x;
            scaled_last_mouse_pos.y *= scale_y;
        }

        let scaled_clicked_pos = clicked_pos.map(|mut pos| {
            pos.x *= scale_x;
            pos.y *= scale_y;
            pos
        });

        // Rebuild metrics every frame so window resizes and DPI changes are handled.
        let metrics = ScreenMetrics::from_dpi(width, height, dpi);

        // 9. Layout pass
        let layout_tree = calculate_layout(&root_element, Size::new(width, height), 0.0, 0.0);

        // 10. Translate pointer events → UiEvent → EventBus
        if mouse_moved {
            let prev_hovered = hovered_btn;
            let hovered_data =
                find_hovered_button(&root_element, &layout_tree, scaled_last_mouse_pos);
            hovered_btn = hovered_data.map(|(id, _)| id);

            match (prev_hovered, hovered_data) {
                (Some(prev), Some((current, rect))) if prev != current => {
                    event_bus.push(UiEvent::HoverEnd(prev));
                    event_bus.push(UiEvent::Hover(
                        current,
                        rect.width,
                        rect.height,
                        scaled_last_mouse_pos.x - rect.x,
                        scaled_last_mouse_pos.y - rect.y,
                    ));
                }
                (Some(prev), Some((current, rect))) if prev == current => {
                    event_bus.push(UiEvent::Hover(
                        current,
                        rect.width,
                        rect.height,
                        scaled_last_mouse_pos.x - rect.x,
                        scaled_last_mouse_pos.y - rect.y,
                    ));
                }
                (None, Some((current, rect))) => {
                    event_bus.push(UiEvent::Hover(
                        current,
                        rect.width,
                        rect.height,
                        scaled_last_mouse_pos.x - rect.x,
                        scaled_last_mouse_pos.y - rect.y,
                    ));
                }
                (Some(prev), None) => {
                    event_bus.push(UiEvent::HoverEnd(prev));
                }
                _ => {} // Same button, no change
            }
        }

        if let Some(clicked_pt) = scaled_clicked_pos {
            // Sürükleme başladığında hangi ScrollView üzerinde olduğumuzu "Capture" ediyoruz
            if let Some((
                crate::core::types::Element::ScrollView {
                    id, capture_drag, ..
                },
                _,
            )) = crate::core::render::draw::find_hovered_scrollview(
                &root_element,
                &layout_tree,
                clicked_pt,
            ) {
                if capture_drag.unwrap_or(true) {
                    active_scrollview_drag = id
                        .as_deref()
                        .map(|id_str| crate::core::ui::widget::fnv1a(id_str.as_bytes()));
                }
                // Mevcut ivmeli kaydırmaları temizle (dokunduğunda dursun)
                kinetic_scrolls.clear();
            }

            if let Some((clicked_btn, rect)) =
                find_clicked_button(&root_element, &layout_tree, clicked_pt)
            {
                event_bus.push(UiEvent::PointerDown(clicked_btn));
                // Push Click to event bus ONLY; plugins will decide the action
                event_bus.push(UiEvent::Click(clicked_btn, rect.width, rect.height));
            } else {
                event_bus.push(UiEvent::PointerDown(0)); // 0 represents nothing
                event_bus.push(UiEvent::ClickOutside);
            }
        }

        if mouse_released {
            if let Some(sv_id) = active_scrollview_drag {
                // Eğer capture varsa, bırakıldığında ivmeyi aktarabiliriz
                if last_drag_delta.0.abs() > 0.5 || last_drag_delta.1.abs() > 0.5 {
                    // Bulalım bakalım momentum açık mı
                    // Hızlıca ağaçta bulmak için active_scrollview_drag kullanılamaz, o yüzden varsayılan true kabul ediyoruz
                    // Ya da basite kaçıp hep true yapabiliriz. Gerçek bir implementasyonda ID ile ağacı taramak gerekir.
                    // Fakat find_hovered_scrollview ile son konumu tararsak bilebiliriz.
                    let momentum_enabled = if let Some((
                        crate::core::types::Element::ScrollView {
                            momentum_scrolling, ..
                        },
                        _,
                    )) =
                        crate::core::render::draw::find_hovered_scrollview(
                            &root_element,
                            &layout_tree,
                            scaled_last_mouse_pos,
                        ) {
                        momentum_scrolling.unwrap_or(true)
                    } else {
                        true // Varsayılan
                    };

                    if momentum_enabled {
                        kinetic_scrolls.push(KineticScroll {
                            sv_id,
                            velocity_x: last_drag_delta.0,
                            velocity_y: last_drag_delta.1,
                        });
                    }
                }
            }

            let released_btn =
                find_hovered_button(&root_element, &layout_tree, scaled_last_mouse_pos)
                    .map(|(id, _)| id);
            event_bus.push(UiEvent::PointerUp(released_btn));

            active_scrollview_drag = None;
            last_drag_delta = (0.0, 0.0);
        }

        for (x, y) in scroll_events {
            if let Some((
                crate::core::types::Element::ScrollView {
                    id,
                    scroll_sensitivity,
                    dynamic_sensitivity,
                    ..
                },
                lay,
            )) = crate::core::render::draw::find_hovered_scrollview(
                &root_element,
                &layout_tree,
                scaled_last_mouse_pos,
            ) {
                let mut factor = scroll_sensitivity.unwrap_or(1.0);
                if dynamic_sensitivity.unwrap_or(false) && !lay.children.is_empty() {
                    let view_height = lay.rect.height;
                    let mut min_y = f32::MAX;
                    let mut max_y = f32::MIN;
                    for child in &lay.children {
                        if child.rect.y < min_y {
                            min_y = child.rect.y;
                        }
                        if child.rect.y + child.rect.height > max_y {
                            max_y = child.rect.y + child.rect.height;
                        }
                    }
                    let content_height = max_y - min_y;
                    if view_height > 0.0 && content_height > view_height {
                        factor *= content_height / view_height;
                    }
                }

                let sv_id = id
                    .as_deref()
                    .map(|id_str| crate::core::ui::widget::fnv1a(id_str.as_bytes()));
                event_bus.push(UiEvent::Scroll(
                    sv_id,
                    f32::from(i16::try_from(x).unwrap_or(0)) * -20.0 * factor,
                    f32::from(i16::try_from(y).unwrap_or(0)) * -20.0 * factor,
                ));
            }
        }

        // Dokunmatik simülasyonu (Sol Tık ile Sürükleme)
        let mut sum_dx = 0.0;
        let mut sum_dy = 0.0;
        for (dx, dy) in drag_events {
            let s_dx = f32::from(i16::try_from(dx).unwrap_or(0)) * scale_x;
            let s_dy = f32::from(i16::try_from(dy).unwrap_or(0)) * scale_y;
            sum_dx += s_dx;
            sum_dy += s_dy;

            if let Some(sv_id) = active_scrollview_drag {
                event_bus.push(UiEvent::Scroll(Some(sv_id), s_dx, s_dy));
            } else if let Some((
                crate::core::types::Element::ScrollView {
                    id, capture_drag, ..
                },
                _,
            )) = crate::core::render::draw::find_hovered_scrollview(
                &root_element,
                &layout_tree,
                scaled_last_mouse_pos,
            ) {
                if capture_drag.unwrap_or(true) && active_scrollview_drag.is_none() {
                    active_scrollview_drag = id
                        .as_deref()
                        .map(|id_str| crate::core::ui::widget::fnv1a(id_str.as_bytes()));
                }
                let sv_id = id
                    .as_deref()
                    .map(|id_str| crate::core::ui::widget::fnv1a(id_str.as_bytes()));
                event_bus.push(UiEvent::Scroll(sv_id, s_dx, s_dy));
            }
        }

        if sum_dx != 0.0 || sum_dy != 0.0 {
            last_drag_delta = (sum_dx, sum_dy);
        }

        // İvmeli kaydırmaları işle
        kinetic_scrolls.retain_mut(|k| {
            if k.velocity_x.abs() > 0.1 || k.velocity_y.abs() > 0.1 {
                event_bus.push(UiEvent::Scroll(Some(k.sv_id), k.velocity_x, k.velocity_y));
                k.velocity_x *= 0.92; // Sürtünme
                k.velocity_y *= 0.92;
                true
            } else {
                false
            }
        });

        // 11. Dispatch queued events to plugins (O(1) per event)
        plugin_registry.dispatch(&event_bus, &style_map, &data_map, &action_queue);

        // Process any actions emitted by plugins
        if let Ok(mut q) = action_queue.lock() {
            for action in q.drain(..) {
                match action {
                    Action::OpenSettings => println!("Desktop Preview: Open Settings triggered"),
                    Action::OpenContacts => println!("Desktop Preview: Open Contacts triggered"),
                    Action::OpenCamera => println!("Desktop Preview: Open Camera triggered"),
                    Action::LoadImage { id, src } => {
                        // Load image with image crate
                        let result = image::open(&src);
                        match result {
                            Ok(img) => {
                                let rgba = img.to_rgba8();
                                let (w, h) = rgba.dimensions();
                                renderer.load_image(&id, rgba.as_raw(), w, h);
                            }
                            Err(e) => println!("Failed to load image {src}: {e}"),
                        }
                    }
                    Action::FocusTextInput(_id) => {
                        video_subsystem.text_input().start();
                    }
                    Action::BlurTextInput => {
                        video_subsystem.text_input().stop();
                    }
                }
            }
        }

        // 12. Render
        renderer.begin_frame(width, height);
        renderer.clear(BACKGROUND);
        // Draw UI tree
        draw_ui(
            &mut renderer,
            &root_element,
            &layout_tree,
            &metrics,
            &style_map,
            &data_map,
            1.0,
        );
        renderer.end_frame();

        window.gl_swap_window();
        std::thread::sleep(Duration::from_millis(16)); // Target ~60 FPS
    }

    Ok(())
}
