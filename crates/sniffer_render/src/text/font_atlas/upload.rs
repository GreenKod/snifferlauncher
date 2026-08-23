use glow::HasContext;

pub(crate) fn upload_font_texture(
    gl: &glow::Context,
    atlas_width: u32,
    atlas_height: u32,
    atlas_pixels: &[u8],
) -> Result<glow::Texture, String> {
    unsafe {
        let tex = gl
            .create_texture()
            .map_err(|_| "Failed to create font texture".to_string())?;
        gl.bind_texture(glow::TEXTURE_2D, Some(tex));

        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);

        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            i32::try_from(glow::RGBA).expect("RGBA fits in i32"),
            i32::try_from(atlas_width).expect("atlas width fits in i32"),
            i32::try_from(atlas_height).expect("atlas height fits in i32"),
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(Some(atlas_pixels)),
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            i32::try_from(glow::LINEAR).expect("LINEAR fits in i32"),
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            i32::try_from(glow::LINEAR).expect("LINEAR fits in i32"),
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

        Ok(tex)
    }
}

pub(crate) fn update_font_texture_sub(
    gl: &glow::Context,
    tex: glow::Texture,
    atlas_width: u32,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    atlas_pixels: &[u8],
) {
    if width == 0 || height == 0 {
        return;
    }
    unsafe {
        gl.bind_texture(glow::TEXTURE_2D, Some(tex));
        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);

        let mut sub_pixels = Vec::with_capacity((width * height * 4) as usize);
        for row in 0..height {
            let src_idx = (((y + row) * atlas_width + x) * 4) as usize;
            let src_end = src_idx + (width * 4) as usize;
            if src_end <= atlas_pixels.len() {
                sub_pixels.extend_from_slice(&atlas_pixels[src_idx..src_end]);
            }
        }

        gl.tex_sub_image_2d(
            glow::TEXTURE_2D,
            0,
            i32::try_from(x).unwrap_or(0),
            i32::try_from(y).unwrap_or(0),
            i32::try_from(width).unwrap_or(0),
            i32::try_from(height).unwrap_or(0),
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(Some(&sub_pixels)),
        );
    }
}
