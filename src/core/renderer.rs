use crate::core::font::{FONT_DATA, FONT_HEIGHT, FONT_WIDTH};
use crate::core::font_atlas::{self, FontAtlas};
use crate::core::geometry::Rect;
use glow::HasContext;

pub trait Renderer {
    fn clear(&mut self, color: u32);
    fn draw_rect(
        &mut self,
        rect: Rect,
        color: u32,
        radius: f32,
        border_width: f32,
        border_color: Option<u32>,
    );
    fn draw_shadow(&mut self, rect: Rect, radius: f32, offset_y: f32, spread: f32, color: u32);
    fn draw_circle(&mut self, cx: f32, cy: f32, radius: f32, color: u32);
    fn draw_text(&mut self, text: &str, x: f32, y: f32, size: f32, color: u32);
    fn begin_frame(&mut self, width: f32, height: f32);
    fn end_frame(&mut self);
}

pub struct GlowRenderer {
    gl: glow::Context,
    quad_vao: glow::VertexArray,
    _quad_vbo: glow::Buffer,
    shape_program: glow::Program,
    text_program: glow::Program,
    font_texture: glow::Texture,
    resolution: (f32, f32),
    font_atlas: Option<FontAtlas>,
    atlas_width: i32,
    atlas_height: i32,
}

fn unpack_color(color: u32) -> [f32; 4] {
    let a = ((color >> 24) & 0xff) as f32 / 255.0;
    let r = ((color >> 16) & 0xff) as f32 / 255.0;
    let g = ((color >> 8) & 0xff) as f32 / 255.0;
    let b = (color & 0xff) as f32 / 255.0;
    // Default alpha to 1.0 if not specified (i.e. color is 0xRRGGBB)
    let a = if a == 0.0 && color != 0 { 1.0 } else { a };
    [r, g, b, a]
}

impl GlowRenderer {
    #![allow(clippy::missing_safety_doc)]
    pub unsafe fn new(gl: glow::Context) -> Result<Self, String> {
        unsafe { Self::with_font(gl, None) }
    }

    pub unsafe fn with_font(gl: glow::Context, font_data: Option<&[u8]>) -> Result<Self, String> {
        unsafe {
            // Vertex coordinates of a simple unit quad (2 triangles)
            let quad_vertices: [f32; 8] = [0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0];

            // 1. Create Quad VAO & VBO
            let quad_vao = gl.create_vertex_array()?;
            gl.bind_vertex_array(Some(quad_vao));
            let quad_vbo = gl.create_buffer()?;
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(quad_vbo));
            let quad_bytes = std::slice::from_raw_parts(
                quad_vertices.as_ptr() as *const u8,
                std::mem::size_of_val(&quad_vertices),
            );
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, quad_bytes, glow::STATIC_DRAW);
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 8, 0);

            // 2. Compile shaders based on target OS
            #[cfg(target_os = "android")]
            let (shape_vs_src, shape_fs_src, text_vs_src, text_fs_src) = (
                include_str!("shaders/shape_android.vs"),
                include_str!("shaders/shape_android.fs"),
                include_str!("shaders/text_android.vs"),
                include_str!("shaders/text_android.fs"),
            );

            #[cfg(not(target_os = "android"))]
            let (shape_vs_src, shape_fs_src, text_vs_src, text_fs_src) = (
                include_str!("shaders/shape_desktop.vs"),
                include_str!("shaders/shape_desktop.fs"),
                include_str!("shaders/text_desktop.vs"),
                include_str!("shaders/text_desktop.fs"),
            );

            let shape_program = compile_program(&gl, shape_vs_src, shape_fs_src)?;
            let text_program = compile_program(&gl, text_vs_src, text_fs_src)?;

            // 3. Create font atlas texture
            let (font_texture, font_atlas, atlas_width, atlas_height) =
                if let Some(font_bytes) = font_data {
                    if let Ok(atlas) = font_atlas::build_font_atlas(&gl, font_bytes, 32.0) {
                        eprintln!(
                            "[DEBUG] Using TTF font atlas ({}x{})",
                            atlas.atlas_width, atlas.atlas_height
                        );
                        let tex = atlas.texture;
                        let w = atlas.atlas_width;
                        let h = atlas.atlas_height;
                        (tex, Some(atlas), w, h)
                    } else {
                        eprintln!("[DEBUG] TTF build failed, falling back to bitmap");
                        create_bitmap_font_atlas(&gl)?
                    }
                } else {
                    eprintln!("[DEBUG] No font data, using bitmap fallback");
                    create_bitmap_font_atlas(&gl)?
                };

            // Enable alpha blending
            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);

            Ok(Self {
                gl,
                quad_vao,
                _quad_vbo: quad_vbo,
                shape_program,
                text_program,
                font_texture,
                resolution: (800.0, 600.0),
                font_atlas,
                atlas_width,
                atlas_height,
            })
        }
    }
}

unsafe fn create_bitmap_font_atlas(
    gl: &glow::Context,
) -> Result<(glow::Texture, Option<FontAtlas>, i32, i32), String> {
    unsafe {
        let mut font_pixels = vec![0u8; 96 * FONT_WIDTH * FONT_HEIGHT];
        for c in 0..96 {
            for row in 0..FONT_HEIGHT {
                let byte = FONT_DATA[c * FONT_HEIGHT + row];
                for col in 0..FONT_WIDTH {
                    let bit = (byte >> (7 - col)) & 1;
                    let pixel_idx = row * (96 * FONT_WIDTH) + (c * FONT_WIDTH + col);
                    font_pixels[pixel_idx] = if bit == 1 { 255 } else { 0 };
                }
            }
        }

        let tex = gl.create_texture()?;
        gl.bind_texture(glow::TEXTURE_2D, Some(tex));
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::R8 as i32,
            (96 * FONT_WIDTH) as i32,
            FONT_HEIGHT as i32,
            0,
            glow::RED,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(Some(&font_pixels)),
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            glow::NEAREST as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            glow::NEAREST as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_S,
            glow::CLAMP_TO_EDGE as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_T,
            glow::CLAMP_TO_EDGE as i32,
        );

        Ok((tex, None, (96 * FONT_WIDTH) as i32, FONT_HEIGHT as i32))
    }
}

unsafe fn compile_program(
    gl: &glow::Context,
    vs_src: &str,
    fs_src: &str,
) -> Result<glow::Program, String> {
    unsafe {
        let vs = gl.create_shader(glow::VERTEX_SHADER)?;
        gl.shader_source(vs, vs_src);
        gl.compile_shader(vs);
        if !gl.get_shader_compile_status(vs) {
            return Err(format!("VS Compile Error: {}", gl.get_shader_info_log(vs)));
        }

        let fs = gl.create_shader(glow::FRAGMENT_SHADER)?;
        gl.shader_source(fs, fs_src);
        gl.compile_shader(fs);
        if !gl.get_shader_compile_status(fs) {
            return Err(format!("FS Compile Error: {}", gl.get_shader_info_log(fs)));
        }

        let program = gl.create_program()?;
        gl.attach_shader(program, vs);
        gl.attach_shader(program, fs);
        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            return Err(format!(
                "Shader Link Error: {}",
                gl.get_program_info_log(program)
            ));
        }

        gl.delete_shader(vs);
        gl.delete_shader(fs);

        Ok(program)
    }
}

impl Renderer for GlowRenderer {
    fn clear(&mut self, color: u32) {
        let col = unpack_color(color);
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
        let col = unpack_color(color);
        let b_col = unpack_color(border_color.unwrap_or(0));
        let _has_border = if border_color.is_some() && border_width > 0.0 {
            1.0f32
        } else {
            0.0f32
        };

        unsafe {
            self.gl.use_program(Some(self.shape_program));
            self.gl.bind_vertex_array(Some(self.quad_vao));

            // Set uniforms
            let loc_res = self
                .gl
                .get_uniform_location(self.shape_program, "u_resolution");
            let loc_rect_pos = self
                .gl
                .get_uniform_location(self.shape_program, "u_rect_pos");
            let loc_rect_size = self
                .gl
                .get_uniform_location(self.shape_program, "u_rect_size");
            let loc_color = self.gl.get_uniform_location(self.shape_program, "u_color");
            let loc_radius = self.gl.get_uniform_location(self.shape_program, "u_radius");
            let loc_border_w = self
                .gl
                .get_uniform_location(self.shape_program, "u_border_width");
            let loc_border_col = self
                .gl
                .get_uniform_location(self.shape_program, "u_border_color");
            let loc_is_circle = self
                .gl
                .get_uniform_location(self.shape_program, "u_is_circle");
            let loc_is_shadow = self
                .gl
                .get_uniform_location(self.shape_program, "u_is_shadow");

            self.gl
                .uniform_2_f32(loc_res.as_ref(), self.resolution.0, self.resolution.1);
            self.gl.uniform_2_f32(loc_rect_pos.as_ref(), rect.x, rect.y);
            self.gl
                .uniform_2_f32(loc_rect_size.as_ref(), rect.width, rect.height);
            self.gl
                .uniform_4_f32(loc_color.as_ref(), col[0], col[1], col[2], col[3]);
            self.gl.uniform_1_f32(loc_radius.as_ref(), radius);
            self.gl.uniform_1_f32(loc_border_w.as_ref(), border_width);
            self.gl.uniform_4_f32(
                loc_border_col.as_ref(),
                b_col[0],
                b_col[1],
                b_col[2],
                b_col[3],
            );
            self.gl.uniform_1_f32(loc_is_circle.as_ref(), 0.0);
            self.gl.uniform_1_f32(loc_is_shadow.as_ref(), 0.0);

            self.gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
        }
    }

    fn draw_shadow(&mut self, rect: Rect, radius: f32, offset_y: f32, spread: f32, color: u32) {
        let col = unpack_color(color);
        unsafe {
            self.gl.use_program(Some(self.shape_program));
            self.gl.bind_vertex_array(Some(self.quad_vao));

            let loc_res = self
                .gl
                .get_uniform_location(self.shape_program, "u_resolution");
            let loc_rect_pos = self
                .gl
                .get_uniform_location(self.shape_program, "u_rect_pos");
            let loc_rect_size = self
                .gl
                .get_uniform_location(self.shape_program, "u_rect_size");
            let loc_color = self.gl.get_uniform_location(self.shape_program, "u_color");
            let loc_radius = self.gl.get_uniform_location(self.shape_program, "u_radius");
            let loc_is_circle = self
                .gl
                .get_uniform_location(self.shape_program, "u_is_circle");
            let loc_is_shadow = self
                .gl
                .get_uniform_location(self.shape_program, "u_is_shadow");
            let loc_shadow_blur = self
                .gl
                .get_uniform_location(self.shape_program, "u_shadow_blur");

            // Shadow is slightly larger and offset down
            let shadow_rect = Rect {
                x: rect.x - spread,
                y: rect.y + offset_y - spread,
                width: rect.width + (spread * 2.0),
                height: rect.height + (spread * 2.0),
            };

            self.gl
                .uniform_2_f32(loc_res.as_ref(), self.resolution.0, self.resolution.1);
            self.gl
                .uniform_2_f32(loc_rect_pos.as_ref(), shadow_rect.x, shadow_rect.y);
            self.gl.uniform_2_f32(
                loc_rect_size.as_ref(),
                shadow_rect.width,
                shadow_rect.height,
            );
            self.gl
                .uniform_4_f32(loc_color.as_ref(), col[0], col[1], col[2], col[3]);
            self.gl.uniform_1_f32(loc_radius.as_ref(), radius + spread);
            self.gl.uniform_1_f32(loc_is_circle.as_ref(), 0.0);
            self.gl.uniform_1_f32(loc_is_shadow.as_ref(), 1.0);
            self.gl
                .uniform_1_f32(loc_shadow_blur.as_ref(), spread * 1.5);

            self.gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
        }
    }

    fn draw_circle(&mut self, cx: f32, cy: f32, radius: f32, color: u32) {
        let col = unpack_color(color);
        unsafe {
            self.gl.use_program(Some(self.shape_program));
            self.gl.bind_vertex_array(Some(self.quad_vao));

            let loc_res = self
                .gl
                .get_uniform_location(self.shape_program, "u_resolution");
            let loc_rect_pos = self
                .gl
                .get_uniform_location(self.shape_program, "u_rect_pos");
            let loc_rect_size = self
                .gl
                .get_uniform_location(self.shape_program, "u_rect_size");
            let loc_color = self.gl.get_uniform_location(self.shape_program, "u_color");
            let loc_radius = self.gl.get_uniform_location(self.shape_program, "u_radius");
            let loc_is_circle = self
                .gl
                .get_uniform_location(self.shape_program, "u_is_circle");
            let loc_is_shadow = self
                .gl
                .get_uniform_location(self.shape_program, "u_is_shadow");

            let rect = Rect {
                x: cx - radius,
                y: cy - radius,
                width: radius * 2.0,
                height: radius * 2.0,
            };

            self.gl
                .uniform_2_f32(loc_res.as_ref(), self.resolution.0, self.resolution.1);
            self.gl.uniform_2_f32(loc_rect_pos.as_ref(), rect.x, rect.y);
            self.gl
                .uniform_2_f32(loc_rect_size.as_ref(), rect.width, rect.height);
            self.gl
                .uniform_4_f32(loc_color.as_ref(), col[0], col[1], col[2], col[3]);
            self.gl.uniform_1_f32(loc_radius.as_ref(), radius);
            self.gl.uniform_1_f32(loc_is_circle.as_ref(), 1.0);
            self.gl.uniform_1_f32(loc_is_shadow.as_ref(), 0.0);

            self.gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
        }
    }

    fn draw_text(&mut self, text: &str, x: f32, y: f32, size: f32, color: u32) {
        let col = unpack_color(color);
        unsafe {
            self.gl.use_program(Some(self.text_program));
            self.gl.bind_vertex_array(Some(self.quad_vao));
            self.gl.active_texture(glow::TEXTURE0);
            self.gl
                .bind_texture(glow::TEXTURE_2D, Some(self.font_texture));

            let loc_res = self
                .gl
                .get_uniform_location(self.text_program, "u_resolution");
            let loc_rect_pos = self
                .gl
                .get_uniform_location(self.text_program, "u_rect_pos");
            let loc_rect_size = self
                .gl
                .get_uniform_location(self.text_program, "u_rect_size");
            let loc_color = self.gl.get_uniform_location(self.text_program, "u_color");
            let loc_uv_start = self
                .gl
                .get_uniform_location(self.text_program, "u_uv_start");
            let loc_uv_end = self.gl.get_uniform_location(self.text_program, "u_uv_end");

            self.gl
                .uniform_2_f32(loc_res.as_ref(), self.resolution.0, self.resolution.1);
            self.gl
                .uniform_4_f32(loc_color.as_ref(), col[0], col[1], col[2], col[3]);

            let _aw = self.atlas_width as f32;
            let _ah = self.atlas_height as f32;

            if let Some(ref atlas) = self.font_atlas {
                let scale = size / atlas.rasterize_size;
                let mut curr_x = x;
                // baseline_y: top of text box + ascent (pixels from top to baseline)
                let baseline_y = y + atlas.ascent * scale;

                let aw = self.atlas_width as f32;
                let ah = self.atlas_height as f32;

                for c in text.chars() {
                    if c == ' ' {
                        curr_x += atlas.space_advance * scale;
                        continue;
                    }

                    let code = c as u32;
                    let idx = if (32..=127).contains(&code) {
                        (code - 32) as usize
                    } else {
                        // fallback: '?' character
                        ('?' as u32 - 32) as usize
                    };

                    let glyph = &atlas.glyphs[idx];
                    if glyph.width == 0 || glyph.height == 0 {
                        curr_x += glyph.advance_width * scale;
                        continue;
                    }

                    let gw = glyph.width as f32 * scale;
                    let gh = glyph.height as f32 * scale;

                    // bearing_y = distance from baseline to TOP of glyph (positive = above)
                    // So the top-left corner of the quad is at baseline_y - bearing_y*scale
                    let draw_x = curr_x + glyph.bearing_x * scale;
                    let draw_y = baseline_y - glyph.bearing_y * scale;

                    // UV coordinates in [0,1] space within atlas texture
                    let uv_x0 = glyph.atlas_x as f32 / aw;
                    let uv_y0 = glyph.atlas_y as f32 / ah;
                    let uv_x1 = (glyph.atlas_x + glyph.width) as f32 / aw;
                    let uv_y1 = (glyph.atlas_y + glyph.height) as f32 / ah;

                    self.gl.uniform_2_f32(loc_rect_pos.as_ref(), draw_x, draw_y);
                    self.gl.uniform_2_f32(loc_rect_size.as_ref(), gw, gh);
                    self.gl.uniform_2_f32(loc_uv_start.as_ref(), uv_x0, uv_y0);
                    self.gl.uniform_2_f32(loc_uv_end.as_ref(), uv_x1, uv_y1);

                    self.gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
                    curr_x += glyph.advance_width * scale;
                }
            } else {
                let char_width = size;
                let char_height = size;
                let gap = size * 0.1;

                let mut curr_x = x;
                for c in text.chars() {
                    let ascii_code = c as u32;
                    let idx = if (32..=127).contains(&ascii_code) {
                        (ascii_code - 32) as f32
                    } else {
                        95.0
                    };

                    self.gl.uniform_2_f32(loc_rect_pos.as_ref(), curr_x, y);
                    self.gl
                        .uniform_2_f32(loc_rect_size.as_ref(), char_width, char_height);
                    self.gl
                        .uniform_2_f32(loc_uv_start.as_ref(), idx / 96.0, 0.0);
                    self.gl
                        .uniform_2_f32(loc_uv_end.as_ref(), (idx + 1.0) / 96.0, 1.0);

                    self.gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
                    curr_x += char_width + gap;
                }
            }
        }
    }

    fn begin_frame(&mut self, width: f32, height: f32) {
        self.resolution = (width, height);
        unsafe {
            self.gl.viewport(0, 0, width as i32, height as i32);
        }
    }

    fn end_frame(&mut self) {
        // Double buffering is handled by windowing wrapper (SDL2 SwapBuffers / EGL SwapBuffers)
    }
}
