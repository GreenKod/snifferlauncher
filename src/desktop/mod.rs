use crate::core::{
    Action, Application, GlowRenderer, LauncherApp, LauncherMessage, LauncherState, Point,
    Renderer, Size, calculate_layout,
    component::{draw_ui, find_clicked_button, find_hovered_button},
    style::{BACKGROUND, WINDOW_HEIGHT, WINDOW_WIDTH},
};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::Duration;

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
    let display = video_subsystem
        .display_bounds(0)
        .unwrap_or(sdl2::rect::Rect::new(
            0,
            0,
            WINDOW_WIDTH as u32,
            WINDOW_HEIGHT as u32,
        ));
    let init_width = (display.width() as f32 * 0.85) as u32;
    let init_height = (display.height() as f32 * 0.85) as u32;
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
        glow::Context::from_loader_function(|s| video_subsystem.gl_get_proc_address(s) as *const _)
    };

    // Audiowide font gömülü olarak binary'ye dahil edildi
    static FONT_BYTES: &[u8] = include_bytes!("../fonts/audiowide.ttf");
    let mut renderer = unsafe { GlowRenderer::with_font(gl, Some(FONT_BYTES))? };
    let mut event_pump = sdl_context.event_pump()?;

    // 4. Initialize UI State
    let mut state = LauncherState::default();
    let mut last_mouse_pos = Point::zero();

    let mut running = true;
    while running {
        let mut clicked_pos = None;
        let mut mouse_moved = false;

        // 5. Poll Events
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
                    last_mouse_pos = Point::new(x as f32, y as f32);
                    mouse_moved = true;
                }
                Event::MouseButtonDown {
                    mouse_btn: sdl2::mouse::MouseButton::Left,
                    x,
                    y,
                    ..
                } => {
                    clicked_pos = Some(Point::new(x as f32, y as f32));
                }
                _ => {}
            }
        }

        // 6. Query current window size each frame (handles resize, DPI changes)
        let (w, h) = window.drawable_size();
        let width = w as f32;
        let height = h as f32;

        // 6. View & Layout Pass
        let root_element = LauncherApp::view(&state);
        let layout_tree = calculate_layout(&root_element, Size::new(width, height), 0.0, 0.0);

        // 7. Event Dispatch / Processing
        if mouse_moved {
            let hovered_btn = find_hovered_button(&root_element, &layout_tree, last_mouse_pos);
            let _action =
                LauncherApp::update(&mut state, LauncherMessage::ButtonHovered(hovered_btn));
        }

        if let Some(clicked_pt) = clicked_pos {
            if let Some(clicked_btn) = find_clicked_button(&root_element, &layout_tree, clicked_pt)
            {
                if let Some(action) =
                    LauncherApp::update(&mut state, LauncherMessage::ButtonClicked(clicked_btn))
                {
                    handle_action(action);
                }
            }
        }

        renderer.begin_frame(width, height);
        renderer.clear(BACKGROUND);

        draw_ui(&mut renderer, &root_element, &layout_tree);

        renderer.end_frame();

        window.gl_swap_window();
        std::thread::sleep(Duration::from_millis(16)); // Target ~60 FPS
    }

    Ok(())
}

fn handle_action(action: Action) {
    match action {
        Action::OpenSettings => {
            println!("Desktop Preview: Open Settings triggered");
        }
        Action::OpenContacts => {
            println!("Desktop Preview: Open Contacts triggered");
        }
        Action::OpenCamera => {
            println!("Desktop Preview: Open Camera triggered");
        }
    }
}
