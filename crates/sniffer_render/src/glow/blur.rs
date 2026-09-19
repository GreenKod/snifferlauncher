use glow::HasContext;

/// Target identifier for ping-pong double-buffered FBO operations.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PingPongTarget {
    A,
    B,
}

impl PingPongTarget {
    /// Returns the opposite target for the next pass.
    #[must_use]
    pub fn other(self) -> Self {
        match self {
            Self::A => Self::B,
            Self::B => Self::A,
        }
    }
}

/// Ping-pong double-buffered Framebuffer Objects and textures
/// for multi-pass image operations such as downsampling and Kawase blur.
pub struct BlurPipeline {
    pub(crate) fbo_a: glow::Framebuffer,
    pub(crate) texture_a: glow::Texture,
    pub(crate) fbo_b: glow::Framebuffer,
    pub(crate) texture_b: glow::Texture,
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) downsample_factor: f32,
}

impl BlurPipeline {
    /// Creates a new `BlurPipeline` allocating two FBOs and two linear-filtered textures.
    ///
    /// # Errors
    ///
    /// Returns an error if GL texture or framebuffer creation fails, or if an FBO is incomplete.
    pub unsafe fn new(
        gl: &glow::Context,
        width: i32,
        height: i32,
        downsample_factor: f32,
    ) -> Result<Self, String> {
        let w = width.max(1);
        let h = height.max(1);

        unsafe {
            let texture_a = Self::create_texture(gl, w, h)?;
            let fbo_a = gl.create_framebuffer()?;
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo_a));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(texture_a),
                0,
            );

            let status_a = gl.check_framebuffer_status(glow::FRAMEBUFFER);
            if status_a != glow::FRAMEBUFFER_COMPLETE {
                gl.delete_texture(texture_a);
                gl.delete_framebuffer(fbo_a);
                return Err(format!("FBO A incomplete: status 0x{status_a:X}"));
            }

            let texture_b = Self::create_texture(gl, w, h)?;
            let fbo_b = gl.create_framebuffer()?;
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo_b));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(texture_b),
                0,
            );

            let status_b = gl.check_framebuffer_status(glow::FRAMEBUFFER);
            if status_b != glow::FRAMEBUFFER_COMPLETE {
                gl.delete_texture(texture_a);
                gl.delete_framebuffer(fbo_a);
                gl.delete_texture(texture_b);
                gl.delete_framebuffer(fbo_b);
                return Err(format!("FBO B incomplete: status 0x{status_b:X}"));
            }

            // Unbind framebuffer back to default
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);

            Ok(Self {
                fbo_a,
                texture_a,
                fbo_b,
                texture_b,
                width: w,
                height: h,
                downsample_factor,
            })
        }
    }

    unsafe fn create_texture(
        gl: &glow::Context,
        width: i32,
        height: i32,
    ) -> Result<glow::Texture, String> {
        unsafe {
            let tex = gl.create_texture()?;
            gl.bind_texture(glow::TEXTURE_2D, Some(tex));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                i32::try_from(glow::RGBA).map_err(|e| e.to_string())?,
                width,
                height,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(None),
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                i32::try_from(glow::LINEAR).map_err(|e| e.to_string())?,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                i32::try_from(glow::LINEAR).map_err(|e| e.to_string())?,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_S,
                i32::try_from(glow::CLAMP_TO_EDGE).map_err(|e| e.to_string())?,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_T,
                i32::try_from(glow::CLAMP_TO_EDGE).map_err(|e| e.to_string())?,
            );
            gl.bind_texture(glow::TEXTURE_2D, None);
            Ok(tex)
        }
    }

    /// Resizes both textures if the requested dimensions differ from current dimensions.
    ///
    /// # Errors
    ///
    /// Returns an error if texture allocation fails.
    pub unsafe fn ensure_size(
        &mut self,
        gl: &glow::Context,
        width: i32,
        height: i32,
    ) -> Result<(), String> {
        let w = width.max(1);
        let h = height.max(1);
        if self.width == w && self.height == h {
            return Ok(());
        }

        unsafe {
            gl.bind_texture(glow::TEXTURE_2D, Some(self.texture_a));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                i32::try_from(glow::RGBA).map_err(|e| e.to_string())?,
                w,
                h,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(None),
            );

            gl.bind_texture(glow::TEXTURE_2D, Some(self.texture_b));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                i32::try_from(glow::RGBA).map_err(|e| e.to_string())?,
                w,
                h,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(None),
            );

            gl.bind_texture(glow::TEXTURE_2D, None);
        }

        self.width = w;
        self.height = h;
        Ok(())
    }

    /// Captures the current backbuffer/framebuffer content into `texture_a` using hardware blit
    /// or fallback texture copy.
    pub unsafe fn capture_screen(&self, gl: &glow::Context, screen_w: i32, screen_h: i32) {
        unsafe {
            // Blit from current READ_FRAMEBUFFER (typically backbuffer 0) to DRAW_FRAMEBUFFER (fbo_a)
            // with hardware linear filtering and downsampling.
            gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(self.fbo_a));
            gl.blit_framebuffer(
                0,
                0,
                screen_w,
                screen_h,
                0,
                0,
                self.width,
                self.height,
                glow::COLOR_BUFFER_BIT,
                glow::LINEAR,
            );
            gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, None);
        }
    }

    /// Returns the FBO for the given target.
    #[must_use]
    pub fn fbo_for(&self, target: PingPongTarget) -> glow::Framebuffer {
        match target {
            PingPongTarget::A => self.fbo_a,
            PingPongTarget::B => self.fbo_b,
        }
    }

    /// Returns the texture for the given target.
    #[must_use]
    pub fn texture_for(&self, target: PingPongTarget) -> glow::Texture {
        match target {
            PingPongTarget::A => self.texture_a,
            PingPongTarget::B => self.texture_b,
        }
    }

    /// Returns the source texture and destination FBO for a ping-pong pass originating from `current`.
    #[must_use]
    pub fn pass_pair(&self, current: PingPongTarget) -> (glow::Texture, glow::Framebuffer) {
        (self.texture_for(current), self.fbo_for(current.other()))
    }

    /// Returns the target texture width.
    #[must_use]
    pub fn width(&self) -> i32 {
        self.width
    }

    /// Returns the target texture height.
    #[must_use]
    pub fn height(&self) -> i32 {
        self.height
    }

    /// Returns the downsampling factor.
    #[must_use]
    pub fn downsample_factor(&self) -> f32 {
        self.downsample_factor
    }

    /// Destroys all GL resources (FBOs and textures).
    pub unsafe fn destroy(&mut self, gl: &glow::Context) {
        unsafe {
            gl.delete_framebuffer(self.fbo_a);
            gl.delete_texture(self.texture_a);
            gl.delete_framebuffer(self.fbo_b);
            gl.delete_texture(self.texture_b);
        }
    }
}
