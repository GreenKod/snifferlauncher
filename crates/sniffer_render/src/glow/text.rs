use super::GlowRenderer;
use super::batching::unpack_color;
use crate::text::font::{FONT_DATA, FONT_HEIGHT, FONT_WIDTH};
use crate::text::font_atlas::FontAtlas;
use glow::HasContext;

pub(crate) unsafe fn create_bitmap_font_atlas(
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
            i32::try_from(glow::R8).expect("R8 fits in i32"),
            i32::try_from(96 * FONT_WIDTH).expect("bitmap atlas width fits in i32"),
            i32::try_from(FONT_HEIGHT).expect("font height fits in i32"),
            0,
            glow::RED,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(Some(&font_pixels)),
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            i32::try_from(glow::NEAREST).expect("NEAREST fits in i32"),
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            i32::try_from(glow::NEAREST).expect("NEAREST fits in i32"),
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_S,
            i32::try_from(glow::CLAMP_TO_EDGE).expect("CLAMP_TO_EDGE fits in i32"),
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_T,
            i32::try_from(glow::CLAMP_TO_EDGE).expect("CLAMP_TO_EDGE fits in i32"),
        );

        Ok((
            tex,
            None,
            i32::try_from(96 * FONT_WIDTH).expect("bitmap atlas width fits in i32"),
            i32::try_from(FONT_HEIGHT).expect("font height fits in i32"),
        ))
    }
}

#[allow(clippy::too_many_lines)]
pub(crate) fn draw_text_impl(
    renderer: &mut GlowRenderer,
    text: &str,
    x: f32,
    y: f32,
    size: f32,
    color: u32,
) {
    if text.is_empty() {
        return;
    }

    if let Some(mut atlas) = renderer.font_atlas.take() {
        atlas.ensure_glyphs(&renderer.gl, text);
        renderer.font_atlas = Some(atlas);
    }

    let mut col = unpack_color(color);
    col[3] *= renderer.global_alpha;

    let Some(t) = renderer.transform_stack.last().copied() else {
        crate::dev_err!("transform_stack empty in draw_text — missing push_transform");
        return;
    };

    let state_changed = renderer
        .text_batch
        .current_transform
        .is_some_and(|mat| mat != t);

    if state_changed || renderer.text_batch.is_full() {
        renderer.flush_text();
    }

    renderer.text_batch.current_transform = Some(t);

    if let Some(ref atlas) = renderer.font_atlas {
        let scale = size / atlas.rasterize_size;
        let mut curr_x = x;
        let baseline_y = atlas.ascent.mul_add(scale, y);

        let aw = f32::from(u16::try_from(renderer.atlas_width).expect("atlas width fits in u16"));
        let ah = f32::from(u16::try_from(renderer.atlas_height).expect("atlas height fits in u16"));

        for c in text.chars() {
            if c == ' ' {
                curr_x = atlas.space_advance.mul_add(scale, curr_x);
                continue;
            }

            let Some(glyph) = atlas.glyphs.get(&c).or_else(|| atlas.glyphs.get(&'?')) else {
                continue;
            };
            if glyph.width == 0 || glyph.height == 0 {
                curr_x = glyph.advance_width.mul_add(scale, curr_x);
                continue;
            }

            let gw =
                f32::from(u16::try_from(glyph.width).expect("glyph width fits in u16")) * scale;
            let gh =
                f32::from(u16::try_from(glyph.height).expect("glyph height fits in u16")) * scale;

            let draw_x = glyph.bearing_x.mul_add(scale, curr_x);
            let draw_y = glyph.bearing_y.mul_add(-scale, baseline_y);

            let u_min_x =
                f32::from(u16::try_from(glyph.atlas_x).expect("atlas x fits in u16")) / aw;
            let u_min_y =
                f32::from(u16::try_from(glyph.atlas_y).expect("atlas y fits in u16")) / ah;
            let u_max_x =
                f32::from(u16::try_from(glyph.atlas_x + glyph.width).expect("atlas x fits in u16"))
                    / aw;
            let u_max_y = f32::from(
                u16::try_from(glyph.atlas_y + glyph.height).expect("atlas y fits in u16"),
            ) / ah;

            let is_col_val = if glyph.is_color { 1.0f32 } else { 0.0f32 };

            // Triangle 1: (top-left, top-right, bottom-left)
            // Triangle 2: (top-right, bottom-right, bottom-left)
            renderer.text_batch.vertices.extend_from_slice(&[
                draw_x,
                draw_y,
                u_min_x,
                u_min_y,
                col[0],
                col[1],
                col[2],
                col[3],
                is_col_val,
                draw_x + gw,
                draw_y,
                u_max_x,
                u_min_y,
                col[0],
                col[1],
                col[2],
                col[3],
                is_col_val,
                draw_x,
                draw_y + gh,
                u_min_x,
                u_max_y,
                col[0],
                col[1],
                col[2],
                col[3],
                is_col_val,
                draw_x + gw,
                draw_y,
                u_max_x,
                u_min_y,
                col[0],
                col[1],
                col[2],
                col[3],
                is_col_val,
                draw_x + gw,
                draw_y + gh,
                u_max_x,
                u_max_y,
                col[0],
                col[1],
                col[2],
                col[3],
                is_col_val,
                draw_x,
                draw_y + gh,
                u_min_x,
                u_max_y,
                col[0],
                col[1],
                col[2],
                col[3],
                is_col_val,
            ]);

            curr_x = glyph.advance_width.mul_add(scale, curr_x);
        }
    } else {
        let char_width = size;
        let char_height = size;
        let gap = size * 0.1;
        let mut curr_x = x;

        for c in text.chars() {
            let ascii_code = c as u32;
            let idx = if (32..=127).contains(&ascii_code) {
                f32::from(u16::try_from(ascii_code - 32).expect("ASCII index fits in u16"))
            } else {
                95.0
            };

            let u_min_x = idx / 96.0;
            let u_max_x = (idx + 1.0) / 96.0;
            let u_min_y = 0.0;
            let u_max_y = 1.0;

            renderer.text_batch.vertices.extend_from_slice(&[
                curr_x,
                y,
                u_min_x,
                u_min_y,
                col[0],
                col[1],
                col[2],
                col[3],
                0.0,
                curr_x + char_width,
                y,
                u_max_x,
                u_min_y,
                col[0],
                col[1],
                col[2],
                col[3],
                0.0,
                curr_x,
                y + char_height,
                u_min_x,
                u_max_y,
                col[0],
                col[1],
                col[2],
                col[3],
                0.0,
                curr_x + char_width,
                y,
                u_max_x,
                u_min_y,
                col[0],
                col[1],
                col[2],
                col[3],
                0.0,
                curr_x + char_width,
                y + char_height,
                u_max_x,
                u_max_y,
                col[0],
                col[1],
                col[2],
                col[3],
                0.0,
                curr_x,
                y + char_height,
                u_min_x,
                u_max_y,
                col[0],
                col[1],
                col[2],
                col[3],
                0.0,
            ]);

            curr_x += char_width + gap;
        }
    }
}

impl GlowRenderer {
    pub fn flush_text(&mut self) {
        if self.text_batch.is_empty() {
            return;
        }

        unsafe {
            self.gl.use_program(Some(self.text_program));
            self.ensure_text_instance_vao();
            self.gl.active_texture(glow::TEXTURE0);
            self.gl
                .bind_texture(glow::TEXTURE_2D, Some(self.font_texture));

            let u = &self.text_uniforms;

            self.gl.uniform_2_f32(
                u.u_resolution.as_ref(),
                self.resolution.0,
                self.resolution.1,
            );
            if let Some(t) = self.text_batch.current_transform {
                self.gl
                    .uniform_matrix_3_f32_slice(u.u_transform.as_ref(), false, &t);
            }

            self.gl
                .bind_buffer(glow::ARRAY_BUFFER, Some(self.text_instance_vbo));

            self.gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                self.text_batch.as_bytes(),
                glow::DYNAMIC_DRAW,
            );
            let vert_count =
                i32::try_from(self.text_batch.len() / 9).expect("vertex count fits in i32");
            self.gl.draw_arrays(glow::TRIANGLES, 0, vert_count);
        }

        self.text_batch.clear();
    }
}
