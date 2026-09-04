use sniffer_render::glow::GlowRenderer;

pub struct EglContextState {
    pub egl: khronos_egl::DynamicInstance,
    pub display: khronos_egl::Display,
    pub config: khronos_egl::Config,
    pub context: khronos_egl::Context,
    pub surface: Option<khronos_egl::Surface>,
    pub renderer: Option<GlowRenderer>,
    pub window: Option<*mut std::ffi::c_void>,
}

impl EglContextState {
    #[allow(clippy::missing_errors_doc)]
    pub fn new() -> Result<Self, String> {
        let egl = unsafe { khronos_egl::DynamicInstance::<khronos_egl::EGL1_5>::load_required() }
            .map_err(|e| format!("Failed to load EGL library: {e}"))?;

        let display = unsafe {
            egl.get_display(khronos_egl::DEFAULT_DISPLAY)
                .ok_or_else(|| "Failed to get EGL display".to_string())?
        };

        egl.initialize(display)
            .map_err(|e| format!("Failed to initialize EGL: {e:?}"))?;

        let attribs_es3 = [
            khronos_egl::SURFACE_TYPE,
            khronos_egl::WINDOW_BIT,
            khronos_egl::RENDERABLE_TYPE,
            khronos_egl::OPENGL_ES3_BIT,
            khronos_egl::BLUE_SIZE,
            8,
            khronos_egl::GREEN_SIZE,
            8,
            khronos_egl::RED_SIZE,
            8,
            khronos_egl::ALPHA_SIZE,
            8,
            khronos_egl::NONE,
        ];

        let (config, version) = match egl.choose_first_config(display, &attribs_es3) {
            Ok(Some(cfg)) => (cfg, 3),
            _ => {
                let attribs_es2 = [
                    khronos_egl::SURFACE_TYPE,
                    khronos_egl::WINDOW_BIT,
                    khronos_egl::RENDERABLE_TYPE,
                    khronos_egl::OPENGL_ES2_BIT,
                    khronos_egl::BLUE_SIZE,
                    8,
                    khronos_egl::GREEN_SIZE,
                    8,
                    khronos_egl::RED_SIZE,
                    8,
                    khronos_egl::ALPHA_SIZE,
                    8,
                    khronos_egl::NONE,
                ];
                let cfg = egl
                    .choose_first_config(display, &attribs_es2)
                    .map_err(|e| format!("Failed to choose EGL config: {e:?}"))?
                    .ok_or_else(|| "No EGL config found".to_string())?;
                (cfg, 2)
            }
        };

        let context_attribs = [
            khronos_egl::CONTEXT_CLIENT_VERSION,
            version,
            khronos_egl::NONE,
        ];

        let context = egl
            .create_context(display, config, None, &context_attribs)
            .map_err(|e| format!("Failed to create EGL context (version {version}): {e:?}"))?;

        Ok(Self {
            egl,
            display,
            config,
            context,
            surface: None,
            renderer: None,
            window: None,
        })
    }

    /// # Safety
    ///
    /// The caller must ensure `native_window_ptr` is a valid pointer to an `ANativeWindow`.
    #[allow(clippy::missing_errors_doc, clippy::not_unsafe_ptr_arg_deref)]
    pub fn bind_window(&mut self, native_window_ptr: *mut std::ffi::c_void) -> Result<(), String> {
        static FONT_BYTES: &[u8] = include_bytes!("../../fonts/audiowide.ttf");

        if native_window_ptr.is_null() {
            return Err("null native window pointer".to_string());
        }

        self.unbind();

        let surface = unsafe {
            ndk_sys::ANativeWindow_acquire(native_window_ptr.cast());
            // 1 = WINDOW_FORMAT_RGBA_8888 (enables hardware alpha channel blending with system wallpaper)
            ndk_sys::ANativeWindow_setBuffersGeometry(native_window_ptr.cast(), 0, 0, 1_i32);

            self.egl
                .create_window_surface(self.display, self.config, native_window_ptr, None)
                .map_err(|e| format!("Failed to create EGL window surface: {e:?}"))?
        };

        self.window = Some(native_window_ptr);

        self.egl
            .make_current(
                self.display,
                Some(surface),
                Some(surface),
                Some(self.context),
            )
            .map_err(|e| format!("Failed to make EGL context current: {e:?}"))?;

        let _ = self.egl.swap_interval(self.display, 1);

        self.surface = Some(surface);

        if self.renderer.is_none() {
            let gl = unsafe {
                glow::Context::from_loader_function(|name| {
                    self.egl
                        .get_proc_address(name)
                        .map_or(std::ptr::null(), |f| f as *const _)
                })
            };
            // Audiowide font is embedded within the binary
            let glow_renderer = unsafe { GlowRenderer::with_font(gl, Some(FONT_BYTES))? };
            self.renderer = Some(glow_renderer);
        }

        Ok(())
    }

    pub fn unbind(&mut self) {
        if let Some(surface) = self.surface.take() {
            let _ = self.egl.make_current(self.display, None, None, None);
            let _ = self.egl.destroy_surface(self.display, surface);
        }
        if let Some(win) = self.window.take() {
            unsafe {
                ndk_sys::ANativeWindow_release(win.cast());
            }
        }
    }

    pub fn swap_buffers(&self) {
        if let Some(surface) = self.surface {
            let _ = self.egl.swap_buffers(self.display, surface);
        }
    }
}

impl Drop for EglContextState {
    fn drop(&mut self) {
        self.unbind();
        let _ = self.egl.destroy_context(self.display, self.context);
        let _ = self.egl.terminate(self.display);
    }
}
