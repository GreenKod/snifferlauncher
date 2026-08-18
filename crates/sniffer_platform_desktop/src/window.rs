use glutin::config::ConfigTemplateBuilder;
use glutin::context::{
    ContextApi, ContextAttributesBuilder, NotCurrentGlContext, PossiblyCurrentContext,
};
use glutin::display::{GetGlDisplay, GlDisplay};
use glutin::prelude::GlSurface;
use glutin::surface::{SurfaceAttributesBuilder, WindowSurface};
use glutin_winit::DisplayBuilder;
use std::num::NonZeroU32;
use std::sync::Arc;
use winit::dpi::LogicalSize;
use winit::event_loop::EventLoop;
use winit::raw_window_handle::HasWindowHandle;
use winit::window::WindowAttributes;

pub struct DesktopWindow {
    pub window: Arc<winit::window::Window>,
    pub gl_surface: glutin::surface::Surface<WindowSurface>,
    pub gl_context: PossiblyCurrentContext,
    pub dpi: f32,
}

impl DesktopWindow {
    #[allow(clippy::missing_panics_doc, clippy::missing_errors_doc)]
    pub fn new() -> Result<(EventLoop<()>, Self, glow::Context), Box<dyn std::error::Error>> {
        let event_loop = EventLoop::new()?;
        let window_icon = {
            let icon_bytes = include_bytes!("../../../android/res/drawable/icon.png");
            if let Ok(img) = image::load_from_memory(icon_bytes) {
                let rgba = img.to_rgba8();
                let (width, height) = rgba.dimensions();
                winit::window::Icon::from_rgba(rgba.into_raw(), width, height).ok()
            } else {
                None
            }
        };

        let mut window_attributes = WindowAttributes::default()
            .with_title("Platform-Agnostic Launcher")
            .with_inner_size(LogicalSize::new(1280.0, 900.0))
            .with_resizable(true);

        if let Some(icon) = window_icon {
            window_attributes = window_attributes.with_window_icon(Some(icon));
        }

        let template = ConfigTemplateBuilder::new().with_alpha_size(8);
        let display_builder = DisplayBuilder::new().with_window_attributes(Some(window_attributes));

        let (window, gl_config) =
            display_builder.build(&event_loop, template, |mut configs| configs.next().unwrap())?;

        let window = window.ok_or("failed to create window")?;
        let gl_display = gl_config.display();

        let size = window.inner_size();
        let surface_attributes = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            window.window_handle().unwrap().as_raw(),
            NonZeroU32::new(size.width).unwrap(),
            NonZeroU32::new(size.height).unwrap(),
        );
        let gl_surface =
            unsafe { gl_display.create_window_surface(&gl_config, &surface_attributes)? };

        let context_attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::OpenGl(Some(glutin::context::Version::new(
                3, 3,
            ))))
            .build(Some(window.window_handle().unwrap().as_raw()));
        let gl_context = unsafe { gl_display.create_context(&gl_config, &context_attributes)? };
        let gl_context = gl_context.make_current(&gl_surface)?;

        let gl = unsafe {
            glow::Context::from_loader_function(|s| {
                let c_string = std::ffi::CString::new(s).unwrap();
                gl_display.get_proc_address(&c_string).cast()
            })
        };

        #[allow(clippy::cast_possible_truncation)]
        let dpi = window.scale_factor() as f32;

        Ok((
            event_loop,
            Self {
                window: Arc::new(window),
                gl_surface,
                gl_context,
                dpi,
            },
            gl,
        ))
    }

    #[must_use]
    pub fn size(&self) -> (u32, u32) {
        let size = self.window.inner_size();
        (size.width, size.height)
    }

    #[must_use]
    pub fn drawable_size(&self) -> (u32, u32) {
        self.size()
    }

    pub fn resize_surface(&self, width: u32, height: u32) {
        if let (Some(w), Some(h)) = (NonZeroU32::new(width), NonZeroU32::new(height)) {
            self.gl_surface.resize(&self.gl_context, w, h);
        }
    }

    /// Swaps the front and back buffers.
    ///
    /// # Errors
    ///
    /// Returns an error if swap buffers fails on the underlying GL surface.
    pub fn swap_buffers(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.gl_surface.swap_buffers(&self.gl_context)?;
        Ok(())
    }
}

