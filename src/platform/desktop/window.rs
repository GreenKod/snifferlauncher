use crate::core::style::{WINDOW_HEIGHT, WINDOW_WIDTH};

pub struct DesktopWindow {
    pub sdl_context: sdl2::Sdl,
    pub video_subsystem: sdl2::VideoSubsystem,
    pub window: sdl2::video::Window,
    pub gl_context: sdl2::video::GLContext,
    pub dpi: f32,
}

impl DesktopWindow {
    #[allow(clippy::missing_panics_doc, clippy::missing_errors_doc)]
    pub fn new() -> Result<(Self, glow::Context), Box<dyn std::error::Error>> {
        let sdl_context = sdl2::init()?;
        sdl2::hint::set("SDL_VIDEO_DRIVER", "x11");
        let video_subsystem = sdl_context.video()?;

        let gl_attr = video_subsystem.gl_attr();
        gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
        gl_attr.set_context_version(3, 3);

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

        let gl_context = window.gl_create_context()?;
        let gl = unsafe {
            glow::Context::from_loader_function(|s| video_subsystem.gl_get_proc_address(s).cast())
        };

        let dpi = video_subsystem
            .display_dpi(0)
            .map_or(96.0, |(_ddpi, hdpi, _vdpi)| hdpi);

        Ok((
            Self {
                sdl_context,
                video_subsystem,
                window,
                gl_context,
                dpi,
            },
            gl,
        ))
    }
}
