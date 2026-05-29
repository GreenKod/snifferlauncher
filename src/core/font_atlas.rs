use glow::HasContext;

pub struct GlyphInfo {
    /// Pixel X start in the atlas texture
    pub atlas_x: u32,
    /// Pixel Y start in the atlas texture (top of glyph)
    pub atlas_y: u32,
    /// Rasterized dimensions in the atlas
    pub width: u32,
    pub height: u32,
    /// Horizontal distance from pen to left edge of glyph
    pub bearing_x: f32,
    /// Distance from baseline to TOP of glyph (positive = above baseline)
    pub bearing_y: f32,
    /// How far to advance the pen after this glyph
    pub advance_width: f32,
}

pub struct FontAtlas {
    pub texture: glow::Texture,
    pub glyphs: Vec<GlyphInfo>,
    pub atlas_width: i32,
    pub atlas_height: i32,
    /// The pixel size used when rasterizing glyphs
    pub rasterize_size: f32,
    /// Ascent from horizontal_line_metrics (baseline to top of line)
    pub ascent: f32,
    /// Space character advance
    pub space_advance: f32,
}

pub fn build_font_atlas(
    gl: &glow::Context,
    font_bytes: &[u8],
    pixel_size: f32,
) -> Result<FontAtlas, String> {
    let font = fontdue::Font::from_bytes(font_bytes, fontdue::FontSettings::default())
        .map_err(|e| format!("Failed to parse font: {:?}", e))?;

    // Gather metrics for each printable ASCII character
    let line_metrics = font
        .horizontal_line_metrics(pixel_size)
        .ok_or("No horizontal line metrics")?;
    let ascent = line_metrics.ascent;

    let space_advance = {
        let idx = font.lookup_glyph_index(' ');
        font.metrics_indexed(idx, pixel_size).advance_width
    };

    // First pass: collect all glyph metrics to compute atlas dimensions
    struct GlyphData {
        width: u32,
        height: u32,
        bearing_x: f32,
        // bearing_y = top above baseline = ymin + height  (fontdue convention)
        bearing_y: f32,
        advance_width: f32,
        bitmap: Vec<u8>,
    }

    let mut glyphs_data: Vec<GlyphData> = Vec::with_capacity(96);
    for c in 32u8..128u8 {
        let (metrics, bitmap) = font.rasterize(c as char, pixel_size);
        let w = metrics.width as u32;
        let h = metrics.height as u32;
        // fontdue: ymin is signed pixels from baseline to bottom of glyph.
        // bearing_y (top edge above baseline) = ymin + height
        let bearing_y = metrics.ymin as f32 + h as f32;
        glyphs_data.push(GlyphData {
            width: w,
            height: h,
            bearing_x: metrics.xmin as f32,
            bearing_y,
            advance_width: metrics.advance_width,
            bitmap,
        });
    }

    // Pack glyphs in a single row: atlas_height = max glyph height
    const PADDING: u32 = 2;
    let atlas_height = glyphs_data.iter().map(|g| g.height).max().unwrap_or(1).max(1) + PADDING * 2;
    let atlas_width = glyphs_data.iter().map(|g| g.width + PADDING * 2).sum::<u32>().max(1);

    let mut atlas_pixels = vec![0u8; (atlas_width * atlas_height) as usize];
    let mut glyph_infos: Vec<GlyphInfo> = Vec::with_capacity(96);
    let mut x_cursor: u32 = 0;

    for g in &glyphs_data {
        let dst_x = x_cursor + PADDING;
        // Place every glyph at Y=PADDING (top of row); bearing_y already accounts for position
        let dst_y = PADDING;

        for row in 0..g.height {
            for col in 0..g.width {
                let src = (row * g.width + col) as usize;
                let dx = dst_x + col;
                let dy = dst_y + row;
                if dx < atlas_width && dy < atlas_height {
                    atlas_pixels[(dy * atlas_width + dx) as usize] = g.bitmap[src];
                }
            }
        }

        glyph_infos.push(GlyphInfo {
            atlas_x: dst_x,
            atlas_y: dst_y,
            width: g.width,
            height: g.height,
            bearing_x: g.bearing_x,
            bearing_y: g.bearing_y,
            advance_width: g.advance_width,
        });

        x_cursor += g.width + PADDING * 2;
    }

    // Upload to GPU
    let font_texture = unsafe {
        let tex = gl
            .create_texture()
            .map_err(|_| "Failed to create font texture".to_string())?;
        gl.bind_texture(glow::TEXTURE_2D, Some(tex));

        // Pixel unpack alignment: our data is 1-byte per pixel
        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);

        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::R8 as i32,
            atlas_width as i32,
            atlas_height as i32,
            0,
            glow::RED,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(Some(&atlas_pixels)),
        );
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
        tex
    };

    eprintln!(
        "[FontAtlas] Built {}x{} atlas, {} glyphs, ascent={:.1}",
        atlas_width, atlas_height, glyph_infos.len(), ascent
    );

    Ok(FontAtlas {
        texture: font_texture,
        glyphs: glyph_infos,
        atlas_width: atlas_width as i32,
        atlas_height: atlas_height as i32,
        rasterize_size: pixel_size,
        ascent,
        space_advance,
    })
}

/// Estimate how wide `text` will render at `text_size`.
pub fn estimate_text_width(atlas: &FontAtlas, text: &str, text_size: f32) -> f32 {
    let scale = text_size / atlas.rasterize_size;
    let mut width = 0.0f32;
    for c in text.chars() {
        if c == ' ' {
            width += atlas.space_advance * scale;
        } else {
            let code = c as u32;
            if code >= 32 && code <= 127 {
                let idx = (code - 32) as usize;
                if idx < atlas.glyphs.len() {
                    width += atlas.glyphs[idx].advance_width * scale;
                }
            }
        }
    }
    width
}
