use sniffer_core::math::Rect;
use sniffer_core::render_api::Renderer;

use crate::dev_err;
use crate::text::font_atlas::{self, FontAtlas};
use glow::HasContext;
use obfstr::obfstr;

pub mod batching;
pub mod shaders;
pub mod text;
pub mod textures;

#[derive(Clone)]
pub struct ShapeUniforms {
    pub u_resolution: Option<glow::UniformLocation>,
    pub u_rect_pos: Option<glow::UniformLocation>,
    pub u_rect_size: Option<glow::UniformLocation>,
    pub u_color: Option<glow::UniformLocation>,
    pub u_radius: Option<glow::UniformLocation>,
    pub u_border_width: Option<glow::UniformLocation>,
    pub u_border_color: Option<glow::UniformLocation>,
    pub u_is_circle: Option<glow::UniformLocation>,
    pub u_is_shadow: Option<glow::UniformLocation>,
    pub u_shadow_blur: Option<glow::UniformLocation>,
    pub u_is_gradient: Option<glow::UniformLocation>,
    pub u_color_bottom: Option<glow::UniformLocation>,
    pub u_shape_size: Option<glow::UniformLocation>,
    pub u_transform: Option<glow::UniformLocation>,
}

#[derive(Clone)]
pub struct ImageUniforms {
    pub u_resolution: Option<glow::UniformLocation>,
    pub u_rect_pos: Option<glow::UniformLocation>,
    pub u_rect_size: Option<glow::UniformLocation>,
    pub u_uv_scale: Option<glow::UniformLocation>,
    pub u_uv_offset: Option<glow::UniformLocation>,
    pub u_radius: Option<glow::UniformLocation>,
    pub u_global_alpha: Option<glow::UniformLocation>,
    pub u_transform: Option<glow::UniformLocation>,
}

#[derive(Clone)]
pub struct TextUniforms {
    pub u_resolution: Option<glow::UniformLocation>,
    pub u_rect_pos: Option<glow::UniformLocation>,
    pub u_rect_size: Option<glow::UniformLocation>,
    pub u_color: Option<glow::UniformLocation>,
    pub u_uv_start: Option<glow::UniformLocation>,
    pub u_uv_end: Option<glow::UniformLocation>,
    pub u_transform: Option<glow::UniformLocation>,
    pub u_is_color: Option<glow::UniformLocation>,
}

pub struct GlowRenderer {
    pub(crate) gl: glow::Context,
    pub(crate) quad_vertex_array: glow::VertexArray,
    pub(crate) _quad_vertex_buffer: glow::Buffer,
    pub(crate) shape_program: glow::Program,
    pub(crate) text_program: glow::Program,
    pub(crate) font_texture: glow::Texture,
    pub(crate) resolution: (f32, f32),
    pub(crate) font_atlas: Option<FontAtlas>,
    pub(crate) atlas_width: i32,
    pub(crate) atlas_height: i32,
    pub(crate) image_program: glow::Program,
    pub(crate) texture_cache: textures::LruTextureCache,
    pub(crate) global_alpha: f32,
    pub(crate) transform_stack: Vec<[f32; 9]>,
    pub(crate) clip_stack: Vec<(Rect, f32)>,
    pub(crate) shape_uniforms: ShapeUniforms,
    pub(crate) image_uniforms: ImageUniforms,
    pub(crate) text_uniforms: TextUniforms,
}

#[allow(clippy::cast_possible_truncation)]
fn f32_to_i32(value: f32) -> i32 {
    value as i32
}

impl GlowRenderer {
    /// Create a renderer using the built-in bitmap font.
    ///
    /// # Errors
    ///
    /// Returns an error if GL object creation, shader compilation, or program linking fails.
    pub unsafe fn new(gl: glow::Context) -> Result<Self, String> {
        unsafe { Self::with_font(gl, None) }
    }

    /// Create a renderer using optional TTF font bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if GL object creation, shader compilation, or program linking fails.
    pub unsafe fn with_font(gl: glow::Context, font_data: Option<&[u8]>) -> Result<Self, String> {
        unsafe {
            // Vertex coordinates of a simple unit quad (2 triangles)
            let quad_vertices: [f32; 8] = [0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0];

            let quad_vertex_array = gl.create_vertex_array()?;
            gl.bind_vertex_array(Some(quad_vertex_array));
            let quad_vertex_buffer = gl.create_buffer()?;
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(quad_vertex_buffer));
            let quad_bytes = std::slice::from_raw_parts(
                quad_vertices.as_ptr().cast::<u8>(),
                std::mem::size_of_val(&quad_vertices),
            );
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, quad_bytes, glow::STATIC_DRAW);
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 8, 0);

            mod enc {
                include!(concat!(env!("OUT_DIR"), "/encrypted_assets.rs"));
            }

            #[cfg(target_os = "android")]
            let (
                shape_vertex_src,
                shape_fragment_src,
                text_vertex_src,
                text_fragment_src,
                image_vertex_src,
                image_fragment_src,
            ) = (
                crate::secure::decrypt(enc::SHAPE_ANDROID_VS),
                crate::secure::decrypt(enc::SHAPE_ANDROID_FS),
                crate::secure::decrypt(enc::TEXT_ANDROID_VS),
                crate::secure::decrypt(enc::TEXT_ANDROID_FS),
                crate::secure::decrypt(enc::IMAGE_ANDROID_VS),
                crate::secure::decrypt(enc::IMAGE_ANDROID_FS),
            );

            #[cfg(not(target_os = "android"))]
            let (
                shape_vertex_src,
                shape_fragment_src,
                text_vertex_src,
                text_fragment_src,
                image_vertex_src,
                image_fragment_src,
            ) = (
                crate::secure::decrypt(enc::SHAPE_DESKTOP_VS),
                crate::secure::decrypt(enc::SHAPE_DESKTOP_FS),
                crate::secure::decrypt(enc::TEXT_DESKTOP_VS),
                crate::secure::decrypt(enc::TEXT_DESKTOP_FS),
                crate::secure::decrypt(enc::IMAGE_DESKTOP_VS),
                crate::secure::decrypt(enc::IMAGE_DESKTOP_FS),
            );

            let shape_program = shaders::compile_program(
                &gl,
                shape_vertex_src.as_str(),
                shape_fragment_src.as_str(),
            )?;
            let text_program = shaders::compile_program(
                &gl,
                text_vertex_src.as_str(),
                text_fragment_src.as_str(),
            )?;
            let image_program = shaders::compile_program(
                &gl,
                image_vertex_src.as_str(),
                image_fragment_src.as_str(),
            )?;

            let (font_texture, font_atlas, atlas_width, atlas_height) =
                if let Some(font_bytes) = font_data {
                    if let Ok(atlas) = font_atlas::build_font_atlas(&gl, font_bytes, 64.0) {
                        dev_err!(
                            "{} ({}x{})",
                            obfstr!("[DEBUG] Using TTF font atlas"),
                            atlas.atlas_width,
                            atlas.atlas_height
                        );
                        let tex = atlas.texture;
                        let w = atlas.atlas_width;
                        let h = atlas.atlas_height;
                        (tex, Some(atlas), w, h)
                    } else {
                        dev_err!(
                            "{}",
                            obfstr!("[DEBUG] TTF build failed, falling back to bitmap")
                        );
                        text::create_bitmap_font_atlas(&gl)?
                    }
                } else {
                    dev_err!("{}", obfstr!("[DEBUG] No font data, using bitmap fallback"));
                    text::create_bitmap_font_atlas(&gl)?
                };

            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);

            let shape_uniforms = ShapeUniforms {
                u_resolution: gl.get_uniform_location(shape_program, "u_resolution"),
                u_rect_pos: gl.get_uniform_location(shape_program, "u_rect_pos"),
                u_rect_size: gl.get_uniform_location(shape_program, "u_rect_size"),
                u_color: gl.get_uniform_location(shape_program, "u_color"),
                u_radius: gl.get_uniform_location(shape_program, "u_radius"),
                u_border_width: gl.get_uniform_location(shape_program, "u_border_width"),
                u_border_color: gl.get_uniform_location(shape_program, "u_border_color"),
                u_is_circle: gl.get_uniform_location(shape_program, "u_is_circle"),
                u_is_shadow: gl.get_uniform_location(shape_program, "u_is_shadow"),
                u_shadow_blur: gl.get_uniform_location(shape_program, "u_shadow_blur"),
                u_is_gradient: gl.get_uniform_location(shape_program, "u_is_gradient"),
                u_color_bottom: gl.get_uniform_location(shape_program, "u_color_bottom"),
                u_shape_size: gl.get_uniform_location(shape_program, "u_shape_size"),
                u_transform: gl.get_uniform_location(shape_program, "u_transform"),
            };

            let image_uniforms = ImageUniforms {
                u_resolution: gl.get_uniform_location(image_program, "u_resolution"),
                u_rect_pos: gl.get_uniform_location(image_program, "u_rect_pos"),
                u_rect_size: gl.get_uniform_location(image_program, "u_rect_size"),
                u_uv_scale: gl.get_uniform_location(image_program, "u_uv_scale"),
                u_uv_offset: gl.get_uniform_location(image_program, "u_uv_offset"),
                u_radius: gl.get_uniform_location(image_program, "u_radius"),
                u_global_alpha: gl.get_uniform_location(image_program, "u_global_alpha"),
                u_transform: gl.get_uniform_location(image_program, "u_transform"),
            };

            let text_uniforms = TextUniforms {
                u_resolution: gl.get_uniform_location(text_program, "u_resolution"),
                u_rect_pos: gl.get_uniform_location(text_program, "u_rect_pos"),
                u_rect_size: gl.get_uniform_location(text_program, "u_rect_size"),
                u_color: gl.get_uniform_location(text_program, "u_color"),
                u_uv_start: gl.get_uniform_location(text_program, "u_uv_start"),
                u_uv_end: gl.get_uniform_location(text_program, "u_uv_end"),
                u_transform: gl.get_uniform_location(text_program, "u_transform"),
                u_is_color: gl.get_uniform_location(text_program, "u_is_color"),
            };

            Ok(Self {
                gl,
                quad_vertex_array,
                _quad_vertex_buffer: quad_vertex_buffer,
                shape_program,
                text_program,
                font_texture,
                resolution: (800.0, 600.0),
                font_atlas,
                atlas_width,
                atlas_height,
                image_program,
                texture_cache: textures::LruTextureCache::default(),
                global_alpha: 1.0,
                transform_stack: vec![[1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]],
                clip_stack: Vec::new(),
                shape_uniforms,
                image_uniforms,
                text_uniforms,
            })
        }
    }

    pub fn trim_memory(&mut self) {
        self.trim_memory_level(textures::MemoryTrimLevel::Critical);
    }

    pub fn warm_up_shaders(&mut self) {
        use sniffer_core::math::Rect;

        let dummy_rect = Rect::new(0.0, 0.0, 1.0, 1.0);
        let alpha_zero = 0x00FFFFFF; // Transparent

        // Use a dummy resolution to avoid zero vectors in shaders
        self.resolution = (1080.0, 1920.0);

        // Warm up all shader variants with a dummy zero-alpha pass
        self.draw_rect_impl(dummy_rect, alpha_zero, 0.0, 0.0, None);
        self.draw_rect_impl(dummy_rect, alpha_zero, 0.0, 1.0, Some(alpha_zero));
        self.draw_rect_impl(dummy_rect, alpha_zero, 10.0, 0.0, None);
        self.draw_rect_gradient_impl(dummy_rect, alpha_zero, alpha_zero, 0.0, 0.0, None);
        self.draw_shadow_impl(dummy_rect, 0.0, 5.0, 10.0, alpha_zero);
        self.draw_circle_impl(0.0, 0.0, 1.0, alpha_zero);
        text::draw_text_impl(self, "W", 0.0, 0.0, 12.0, alpha_zero);

        // Warm up GPU state changes
        unsafe {
            self.gl.enable(glow::SCISSOR_TEST);
            self.gl.scissor(0, 0, 1, 1);
            self.gl.disable(glow::SCISSOR_TEST);
        }
    }
}

impl Renderer for GlowRenderer {
    fn clear(&mut self, color: u32) {
        let col = batching::unpack_color(color);
        unsafe {
            self.gl.clear_color(col[0], col[1], col[2], col[3]);
            self.gl.clear(glow::COLOR_BUFFER_BIT);
        }
    }

    fn draw_rect(
        &mut self,
        rect: Rect,
        color: u32,
        radius: f32,
        border_width: f32,
        border_color: Option<u32>,
    ) {
        self.draw_rect_impl(rect, color, radius, border_width, border_color);
    }

    fn draw_rect_gradient(
        &mut self,
        rect: Rect,
        color_top: u32,
        color_bottom: u32,
        radius: f32,
        border_width: f32,
        border_color: Option<u32>,
    ) {
        self.draw_rect_gradient_impl(
            rect,
            color_top,
            color_bottom,
            radius,
            border_width,
            border_color,
        );
    }

    fn draw_shadow(&mut self, rect: Rect, radius: f32, offset_y: f32, spread: f32, color: u32) {
        self.draw_shadow_impl(rect, radius, offset_y, spread, color);
    }

    fn draw_circle(&mut self, cx: f32, cy: f32, radius: f32, color: u32) {
        self.draw_circle_impl(cx, cy, radius, color);
    }

    fn draw_text(&mut self, text: &str, x: f32, y: f32, size: f32, color: u32) {
        text::draw_text_impl(self, text, x, y, size, color);
    }

    fn begin_frame(&mut self, width: f32, height: f32) {
        self.resolution = (width, height);
        self.texture_cache.current_frame = self.texture_cache.current_frame.wrapping_add(1);
        unsafe {
            self.gl
                .viewport(0, 0, f32_to_i32(width), f32_to_i32(height));
        }
    }

    fn end_frame(&mut self) {}

    fn set_clip_rect(&mut self, rect: Rect) {
        unsafe {
            self.gl.enable(glow::SCISSOR_TEST);
            let y = self.resolution.1 - rect.y - rect.height;
            self.gl.scissor(
                f32_to_i32(rect.x),
                f32_to_i32(y),
                f32_to_i32(rect.width),
                f32_to_i32(rect.height),
            );
        }
    }

    fn clear_clip_rect(&mut self) {
        unsafe {
            self.gl.disable(glow::SCISSOR_TEST);
        }
    }

    fn push_clip_rect(&mut self, rect: Rect, radius: f32) {
        let current = if let Some(&(cur_rect, _)) = self.clip_stack.last() {
            let cx = cur_rect.x.max(rect.x);
            let cy = cur_rect.y.max(rect.y);
            let cw = (cur_rect.x + cur_rect.width).min(rect.x + rect.width) - cx;
            let ch = (cur_rect.y + cur_rect.height).min(rect.y + rect.height) - cy;
            Rect::new(cx, cy, cw.max(0.0), ch.max(0.0))
        } else {
            rect
        };
        self.clip_stack.push((current, radius));
        self.set_clip_rect(current);
    }

    fn pop_clip_rect(&mut self) {
        self.clip_stack.pop();
        if let Some(&(rect, _)) = self.clip_stack.last() {
            self.set_clip_rect(rect);
        } else {
            self.clear_clip_rect();
        }
    }

    fn push_transform(&mut self, cx: f32, cy: f32, scale: f32, rotate: f32, tx: f32, ty: f32) {
        let p = self
            .transform_stack
            .last()
            .copied()
            .unwrap_or([1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);

        let rot_rad = rotate.to_radians();
        let c = rot_rad.cos() * scale;
        let s = rot_rad.sin() * scale;

        let b0 = c;
        let b1 = s;
        let b3 = -s;
        let b4 = c;

        let b6 = cx + tx - (c * cx - s * cy);
        let b7 = cy + ty - (s * cx + c * cy);

        let res = [
            b1.mul_add(p[3], p[0] * b0),
            b1.mul_add(p[4], p[1] * b0),
            b1.mul_add(p[5], p[2] * b0),
            b4.mul_add(p[3], p[0] * b3),
            b4.mul_add(p[4], p[1] * b3),
            b4.mul_add(p[5], p[2] * b3),
            p[6] + b7.mul_add(p[3], p[0] * b6),
            p[7] + b7.mul_add(p[4], p[1] * b6),
            p[8] + b7.mul_add(p[5], p[2] * b6),
        ];

        self.transform_stack.push(res);
    }

    fn pop_transform(&mut self) {
        if self.transform_stack.len() > 1 {
            self.transform_stack.pop();
        }
    }

    fn set_global_alpha(&mut self, alpha: f32) {
        self.global_alpha = alpha;
    }

    fn load_image(&mut self, id: &str, rgba_pixels: &[u8], width: u32, height: u32) {
        self.load_image_impl(id, rgba_pixels, width, height);
    }

    fn has_image(&self, id: &str) -> bool {
        self.has_image_impl(id)
    }

    fn load_wallpaper(&mut self, rgba_pixels: &[u8], width: u32, height: u32) {
        self.load_wallpaper_impl(rgba_pixels, width, height);
    }

    fn draw_wallpaper(&mut self, width: f32, height: f32) {
        self.draw_wallpaper_impl(width, height);
    }

    fn draw_image(
        &mut self,
        id: &str,
        rect: Rect,
        radius: f32,
        object_fit: sniffer_core::style::ObjectFit,
    ) {
        self.draw_image_impl(id, rect, radius, object_fit);
    }

    fn measure_text(&self, text: &str, size: f32) -> f32 {
        self.font_atlas.as_ref().map_or_else(
            || {
                let char_width = size;
                let gap = size * 0.1;
                #[allow(clippy::cast_precision_loss)]
                let count = text.chars().count() as f32;
                if count > 0.0 {
                    (char_width + gap).mul_add(count, -gap)
                } else {
                    0.0
                }
            },
            |atlas| font_atlas::estimate_text_width(atlas, text, size),
        )
    }

    fn text_ascent(&self, size: f32) -> f32 {
        self.font_atlas.as_ref().map_or(size * 0.75, |atlas| {
            let scale = size / atlas.rasterize_size;
            atlas.ascent * scale
        })
    }
}
