use sniffer_core::math::Rect;

use crate::dev_err;
use crate::text::font_atlas::{self, FontAtlas};
use glow::HasContext;
use obfstr::obfstr;

pub mod batching;
pub mod renderer_impl;
pub mod shaders;
pub mod text;
pub mod textures;
pub mod uniforms;

pub use uniforms::{ImageUniforms, ShapeUniforms, TextUniforms};

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
pub(crate) fn f32_to_i32(value: f32) -> i32 {
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
                        let (t, opt_atlas, w, h) = text::create_bitmap_font_atlas(&gl)?;
                        (t, opt_atlas, w, h)
                    }
                } else {
                    let (t, opt_atlas, w, h) = text::create_bitmap_font_atlas(&gl)?;
                    (t, opt_atlas, w, h)
                };

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

            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);

            let default_matrix = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];

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
                transform_stack: vec![default_matrix],
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
        let dummy_rect = Rect::new(0.0, 0.0, 1.0, 1.0);
        let alpha_zero = 0x00FF_FFFF;

        self.resolution = (1080.0, 1920.0);

        self.draw_rect_impl(dummy_rect, alpha_zero, 0.0, 0.0, None);
        self.draw_rect_impl(dummy_rect, alpha_zero, 0.0, 1.0, Some(alpha_zero));
        self.draw_rect_impl(dummy_rect, alpha_zero, 10.0, 0.0, None);
        self.draw_rect_gradient_impl(dummy_rect, alpha_zero, alpha_zero, 0.0, 0.0, None);
        self.draw_shadow_impl(dummy_rect, 0.0, 5.0, 10.0, alpha_zero);
        self.draw_circle_impl(0.0, 0.0, 1.0, alpha_zero);
        text::draw_text_impl(self, "W", 0.0, 0.0, 12.0, alpha_zero);

        unsafe {
            self.gl.enable(glow::SCISSOR_TEST);
            self.gl.scissor(0, 0, 1, 1);
            self.gl.disable(glow::SCISSOR_TEST);
        }
    }

    /// Captures the current OpenGL framebuffer into a PNG file.
    ///
    /// # Errors
    ///
    /// Returns an error if reading pixels or saving the PNG fails.
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    pub fn capture_framebuffer_png<P: AsRef<std::path::Path>>(
        &self,
        width: u32,
        height: u32,
        output_path: P,
    ) -> Result<(), String> {
        let (w_usize, h_usize) = (width as usize, height as usize);
        let mut pixels = vec![0u8; w_usize * h_usize * 4];
        unsafe {
            self.gl.read_pixels(
                0,
                0,
                width as i32,
                height as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut pixels)),
            );
        }

        // Flip vertically: OpenGL framebuffer starts at bottom-left
        let row_bytes = w_usize * 4;
        let mut flipped = vec![0u8; pixels.len()];
        for y in 0..h_usize {
            let src_start = y * row_bytes;
            let src_end = src_start + row_bytes;
            let dst_start = (h_usize - 1 - y) * row_bytes;
            let dst_end = dst_start + row_bytes;
            flipped[dst_start..dst_end].copy_from_slice(&pixels[src_start..src_end]);
        }

        image::save_buffer(
            output_path,
            &flipped,
            width,
            height,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| format!("Failed to save screenshot PNG: {e}"))
    }
}
