use crate::core::render::text::font::{FONT_DATA, FONT_HEIGHT, FONT_WIDTH};
use crate::core::render::text::font_atlas::FontAtlas;
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
