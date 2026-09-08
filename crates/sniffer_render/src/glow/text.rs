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
    if let Some(mut atlas) = renderer.font_atlas.take() {
        atlas.ensure_glyphs(&renderer.gl, text);
        renderer.font_atlas = Some(atlas);
    }

    let mut col = unpack_color(color);
    col[3] *= renderer.global_alpha;
    unsafe {
        renderer.gl.use_program(Some(renderer.text_program));
        renderer.ensure_quad_vao();
        renderer.gl.active_texture(glow::TEXTURE0);
        renderer
            .gl
            .bind_texture(glow::TEXTURE_2D, Some(renderer.font_texture));

        let u = renderer.text_uniforms.clone();

        renderer.gl.uniform_2_f32(
            u.u_resolution.as_ref(),
            renderer.resolution.0,
            renderer.resolution.1,
        );
        renderer
            .gl
            .uniform_4_f32(u.u_color.as_ref(), col[0], col[1], col[2], col[3]);

        if let Some(ref atlas) = renderer.font_atlas {
            let scale = size / atlas.rasterize_size;
            let mut curr_x = x;
            let baseline_y = atlas.ascent.mul_add(scale, y);

            let aw =
                f32::from(u16::try_from(renderer.atlas_width).expect("atlas width fits in u16"));
            let ah =
                f32::from(u16::try_from(renderer.atlas_height).expect("atlas height fits in u16"));

            for c in text.chars() {
                if c == ' ' {
                    curr_x = atlas.space_advance.mul_add(scale, curr_x);
                    continue;
                }

                let glyph = atlas
                    .glyphs
                    .get(&c)
                    .unwrap_or_else(|| atlas.glyphs.get(&'?').unwrap());
                if glyph.width == 0 || glyph.height == 0 {
                    curr_x = glyph.advance_width.mul_add(scale, curr_x);
                    continue;
                }

                let is_color_val = i32::from(glyph.is_color);
                renderer
                    .gl
                    .uniform_1_i32(u.u_is_color.as_ref(), is_color_val);

                let gw =
                    f32::from(u16::try_from(glyph.width).expect("glyph width fits in u16")) * scale;
                let gh = f32::from(u16::try_from(glyph.height).expect("glyph height fits in u16"))
                    * scale;

                let draw_x = glyph.bearing_x.mul_add(scale, curr_x);
                let draw_y = glyph.bearing_y.mul_add(-scale, baseline_y);

                let u_min_x =
                    f32::from(u16::try_from(glyph.atlas_x).expect("atlas x fits in u16")) / aw;
                let u_min_y =
                    f32::from(u16::try_from(glyph.atlas_y).expect("atlas y fits in u16")) / ah;
                let u_max_x = f32::from(
                    u16::try_from(glyph.atlas_x + glyph.width).expect("atlas x fits in u16"),
                ) / aw;
                let u_max_y = f32::from(
                    u16::try_from(glyph.atlas_y + glyph.height).expect("atlas y fits in u16"),
                ) / ah;

                renderer
                    .gl
                    .uniform_2_f32(u.u_rect_pos.as_ref(), draw_x, draw_y);
                renderer.gl.uniform_2_f32(u.u_rect_size.as_ref(), gw, gh);
                renderer
                    .gl
                    .uniform_2_f32(u.u_uv_start.as_ref(), u_min_x, u_min_y);
                renderer
                    .gl
                    .uniform_2_f32(u.u_uv_end.as_ref(), u_max_x, u_max_y);

                let t = renderer.transform_stack.last().unwrap();
                renderer
                    .gl
                    .uniform_matrix_3_f32_slice(u.u_transform.as_ref(), false, t);
                renderer.gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
                curr_x = glyph.advance_width.mul_add(scale, curr_x);
            }
        } else {
            renderer.gl.uniform_1_i32(u.u_is_color.as_ref(), 0);
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

                renderer.gl.uniform_2_f32(u.u_rect_pos.as_ref(), curr_x, y);
                renderer
                    .gl
                    .uniform_2_f32(u.u_rect_size.as_ref(), char_width, char_height);
                renderer
                    .gl
                    .uniform_2_f32(u.u_uv_start.as_ref(), idx / 96.0, 0.0);
                renderer
                    .gl
                    .uniform_2_f32(u.u_uv_end.as_ref(), (idx + 1.0) / 96.0, 1.0);

                let t = renderer.transform_stack.last().unwrap();
                renderer
                    .gl
                    .uniform_matrix_3_f32_slice(u.u_transform.as_ref(), false, t);
                renderer.gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
                curr_x += char_width + gap;
            }
        }
    }
}
