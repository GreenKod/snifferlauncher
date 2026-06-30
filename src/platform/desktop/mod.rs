use crate::core::render::draw::{draw_ui, find_clicked_button, find_hovered_button};
use crate::core::style::{BACKGROUND, WINDOW_HEIGHT, WINDOW_WIDTH};
use crate::core::ui::data_map::DataMap;
use crate::core::ui::event::{EventBus, UiEvent};
use crate::core::ui::style_map::StyleMap;
use crate::core::{
    Action, Application, GlowRenderer, LauncherApp, LauncherMessage, LauncherState, Point,
    Renderer, ScreenMetrics, Size, calculate_layout,
};
use crate::plugin::registry::PluginRegistry;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::Duration;

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
#[allow(clippy::too_many_lines)]
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

    // 5. Initialize UI state
    let mut state = LauncherState::default();
    let mut last_mouse_pos = Point::zero();

    // 6. Initialize the event-driven plugin system
    let event_bus = EventBus::default();
    let style_map = StyleMap::default();
    let data_map = DataMap::default();

    let mut plugin_registry = PluginRegistry::default();

    // Instead of hardcoding, we load from .plugins/plugins.json
    let loader = crate::plugin::PluginLoader::new(".plugins");
    loader.register_all(&mut plugin_registry);

    let mut running = true;
    while running {
        let mut clicked_pos = None;
        let mut mouse_moved = false;

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
                Event::MouseMotion { x, y, .. } => {
                    last_mouse_pos = Point::new(
                        f32::from(i16::try_from(x).expect("mouse x fits in i16")),
                        f32::from(i16::try_from(y).expect("mouse y fits in i16")),
                    );
                    mouse_moved = true;
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
                _ => {}
            }
        }

        // 8. Query current drawable size each frame (handles resize and DPI changes)
        let (w, h) = window.drawable_size();
        let width = f32::from(u16::try_from(w).expect("drawable width fits in u16"));
        let height = f32::from(u16::try_from(h).expect("drawable height fits in u16"));

        // Rebuild metrics every frame so window resizes and DPI changes are handled.
        let metrics = ScreenMetrics::from_dpi(width, height, dpi);

        // 9. View & Layout pass
        let root_element = LauncherApp::view(&state, &metrics);
        let layout_tree = calculate_layout(&root_element, Size::new(width, height), 0.0, 0.0);

        // 10. Translate pointer events → UiEvent → EventBus
        if mouse_moved {
            let prev_hovered = state.hovered;
            let hovered_btn = find_hovered_button(&root_element, &layout_tree, last_mouse_pos);

            let _ = LauncherApp::update(&mut state, LauncherMessage::ButtonHovered(hovered_btn));

            match (prev_hovered, hovered_btn) {
                (Some(prev), Some(current)) if prev != current => {
                    // Moved from one button to another
                    event_bus.push(UiEvent::HoverEnd(
                        crate::core::ui::widget::ids::from_button_id(prev),
                    ));
                    event_bus.push(UiEvent::Hover(
                        crate::core::ui::widget::ids::from_button_id(current),
                    ));
                }
                (None, Some(current)) => {
                    // Entered a button
                    event_bus.push(UiEvent::Hover(
                        crate::core::ui::widget::ids::from_button_id(current),
                    ));
                }
                (Some(prev), None) => {
                    // Left all buttons
                    event_bus.push(UiEvent::HoverEnd(
                        crate::core::ui::widget::ids::from_button_id(prev),
                    ));
                }
                _ => {} // Same button, no change
            }
        }

        if let Some(clicked_pt) = clicked_pos
            && let Some(clicked_btn) = find_clicked_button(&root_element, &layout_tree, clicked_pt)
        {
            // Push Click to event bus before updating state
            let id = crate::core::ui::widget::ids::from_button_id(clicked_btn);
            event_bus.push(UiEvent::Click(id));

            if let Some(action) =
                LauncherApp::update(&mut state, LauncherMessage::ButtonClicked(clicked_btn))
            {
                handle_action(action);
            }
        }

        // 11. Dispatch queued events to plugins (O(1) per event)
        plugin_registry.dispatch(&event_bus, &style_map, &data_map);

        // 12. Render
        renderer.begin_frame(width, height);
        renderer.clear(BACKGROUND);
        draw_ui(
            &mut renderer,
            &root_element,
            &layout_tree,
            &metrics,
            &style_map,
            &data_map,
        );
        renderer.end_frame();

        window.gl_swap_window();
        std::thread::sleep(Duration::from_millis(16)); // Target ~60 FPS
    }

    Ok(())
}

fn handle_action(action: Action) {
    match action {
        Action::OpenSettings => println!("Desktop Preview: Open Settings triggered"),
        Action::OpenContacts => println!("Desktop Preview: Open Contacts triggered"),
        Action::OpenCamera => println!("Desktop Preview: Open Camera triggered"),
    }
}
