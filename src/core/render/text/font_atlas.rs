#![allow(clippy::pedantic, clippy::nursery)]

use glow::HasContext;
use swash::FontRef;
use swash::scale::{Render, ScaleContext, Source, image::Content};

const PADDING: u32 = 2;

struct GlyphData {
    width: u32,
    height: u32,
    bearing_x: f32,
    bearing_y: f32,
    advance_width: f32,
    bitmap: Vec<u8>,
}

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
    pub glyphs: std::collections::HashMap<char, GlyphInfo>,
    pub atlas_width: i32,
    pub atlas_height: i32,
    /// The pixel size used when rasterizing glyphs
    pub rasterize_size: f32,
    /// Ascent from `horizontal_line_metrics` (baseline to top of line).
    pub ascent: f32,
    /// Space character advance
    pub space_advance: f32,
}

/// Build a font atlas texture and metrics from a TTF payload.
///
/// # Errors
///
/// Returns an error if the font cannot be parsed or the GPU texture upload fails.
///
/// # Panics
///
/// Panics if atlas dimensions or glyph metrics do not fit in the narrower integer
/// types required by the GL upload path.
#[allow(clippy::missing_panics_doc)]
pub fn build_font_atlas(
    gl: &glow::Context,
    font_bytes: &[u8],
    pixel_size: f32,
) -> Result<FontAtlas, String> {
    let font =
        FontRef::from_index(font_bytes, 0).ok_or_else(|| "Failed to parse font".to_string())?;

    let font_metrics = font.metrics(&[]);
    let units_per_em = f32::from(font_metrics.units_per_em);
    let scale_factor = pixel_size / units_per_em;
    let ascent = font_metrics.ascent * scale_factor;

    let space_advance = {
        let charmap = font.charmap();
        let glyph_id = charmap.map(' ');
        let glyph_metrics = font.glyph_metrics(&[]);
        glyph_metrics.advance_width(glyph_id) * scale_factor
    };

    let glyphs_data = collect_glyph_data(&font, pixel_size, scale_factor);

    let atlas_height = glyphs_data
        .iter()
        .map(|(_, g)| g.height)
        .max()
        .unwrap_or(1)
        .max(1)
        + PADDING * 2;
    let atlas_width = glyphs_data
        .iter()
        .map(|(_, g)| g.width + PADDING * 2)
        .sum::<u32>()
        .max(1);

    let mut atlas_pixels = vec![
        0u8;
        usize::try_from(atlas_width * atlas_height)
            .expect("atlas dimensions fit in usize")
    ];
    let mut glyph_infos = std::collections::HashMap::with_capacity(128);
    let mut x_cursor: u32 = 0;

    for (character, g) in &glyphs_data {
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

        glyph_infos.insert(*character, GlyphInfo {
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

    let font_texture = upload_font_texture(gl, atlas_width, atlas_height, &atlas_pixels)?;

    eprintln!(
        "[FontAtlas] Built {}x{} atlas, {} glyphs, ascent={:.1}",
        atlas_width,
        atlas_height,
        glyph_infos.len(),
        ascent
    );

    Ok(FontAtlas {
        texture: font_texture,
        glyphs: glyph_infos,
        atlas_width: i32::try_from(atlas_width).expect("atlas width fits in i32"),
        atlas_height: i32::try_from(atlas_height).expect("atlas height fits in i32"),
        rasterize_size: pixel_size,
        ascent,
        space_advance,
    })
}

fn collect_glyph_data(font: &FontRef, pixel_size: f32, scale_factor: f32) -> Vec<(char, GlyphData)> {
    let mut context = ScaleContext::new();
    let mut scaler = context.builder(*font).size(pixel_size).hint(true).build();

    let renderer = Render::new(&[
        Source::Outline,
        Source::Bitmap(swash::scale::StrikeWith::Index(0)),
    ]);

    let charmap = font.charmap();
    let glyph_metrics = font.glyph_metrics(&[]);

    let mut chars_to_render = Vec::new();
    for c in 32u8..128u8 {
        chars_to_render.push(char::from(c));
    }
    for &c in &['ç', 'Ç', 'ğ', 'Ğ', 'ı', 'İ', 'ö', 'Ö', 'ş', 'Ş', 'ü', 'Ü'] {
        chars_to_render.push(c);
    }

    let mut glyphs_data = Vec::with_capacity(128);
    for character in chars_to_render {
        let glyph_id = charmap.map(character);

        let advance_width = glyph_metrics.advance_width(glyph_id) * scale_factor;

        let image = renderer.render(&mut scaler, glyph_id);

        let glyph_data = match image {
            Some(img) => {
                let w = img.placement.width;
                let h = img.placement.height;
                #[allow(clippy::cast_precision_loss)]
                let bx = img.placement.left as f32;
                #[allow(clippy::cast_precision_loss)]
                let by = img.placement.top as f32;

                let data = match img.content {
                    Content::Mask => img.data,
                    Content::SubpixelMask => {
                        let mut gray = Vec::with_capacity(img.data.len() / 3);
                        for chunk in img.data.chunks_exact(3) {
                            let g =
                                (u32::from(chunk[0]) + u32::from(chunk[1]) + u32::from(chunk[2]))
                                    / 3;
                            #[allow(clippy::cast_possible_truncation)]
                            gray.push(g as u8);
                        }
                        gray
                    }
                    Content::Color => {
                        let mut gray = Vec::with_capacity(img.data.len() / 4);
                        for chunk in img.data.chunks_exact(4) {
                            gray.push(chunk[3]);
                        }
                        gray
                    }
                };

                // Generate SDF from the rasterized bitmap
                let spread = 8;
                let (sdf_bitmap, sdf_w, sdf_h) = generate_sdf(&data, w, h, spread);
                
                // Adjust bearings for the padded SDF size
                let sdf_bx = bx - spread as f32;
                let sdf_by = by + spread as f32;

                GlyphData {
                    width: sdf_w,
                    height: sdf_h,
                    bearing_x: sdf_bx,
                    bearing_y: sdf_by,
                    advance_width,
                    bitmap: sdf_bitmap,
                }
            }
            None => GlyphData {
                width: 0,
                height: 0,
                bearing_x: 0.0,
                bearing_y: 0.0,
                advance_width,
                bitmap: Vec::new(),
            },
        };

        glyphs_data.push((character, glyph_data));
    }
    glyphs_data
}

fn generate_sdf(bitmap: &[u8], width: u32, height: u32, spread: u32) -> (Vec<u8>, u32, u32) {
    if width == 0 || height == 0 {
        return (Vec::new(), 0, 0);
    }
    
    let p_width = width + 2 * spread;
    let p_height = height + 2 * spread;
    let mut padded = vec![0u8; (p_width * p_height) as usize];
    
    // Copy bitmap to center of padded buffer
    for y in 0..height {
        for x in 0..width {
            let src = (y * width + x) as usize;
            let dst = ((y + spread) * p_width + (x + spread)) as usize;
            padded[dst] = bitmap[src];
        }
    }
    
    let mut sdf = vec![0u8; (p_width * p_height) as usize];
    let spread_f = spread as f32;
    let spread_i = spread as isize;
    
    for y in 0..p_height as isize {
        for x in 0..p_width as isize {
            let idx = (y * p_width as isize + x) as usize;
            let inside = padded[idx] > 127;
            let mut min_dist_sq = spread_f * spread_f;
            
            let start_dy = (-spread_i).max(-y);
            let end_dy = spread_i.min((p_height as isize) - 1 - y);
            let start_dx = (-spread_i).max(-x);
            let end_dx = spread_i.min((p_width as isize) - 1 - x);

            for dy in start_dy..=end_dy {
                for dx in start_dx..=end_dx {
                    let ny = (y + dy) as usize;
                    let nx = (x + dx) as usize;
                    let n_idx = ny * p_width as usize + nx;
                    let n_inside = padded[n_idx] > 127;
                    
                    if inside != n_inside {
                        let dist_sq = (dx * dx + dy * dy) as f32;
                        if dist_sq < min_dist_sq {
                            min_dist_sq = dist_sq;
                        }
                    }
                }
            }
            
            let min_dist = min_dist_sq.sqrt();
            let dist = if inside { min_dist } else { -min_dist };
            
            // Map [-spread, spread] to [0, 255] where 127.5 is the exact edge
            let norm = 0.5 + 0.5 * (dist / spread_f);
            
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let val = (norm * 255.0).clamp(0.0, 255.0) as u8;
            sdf[idx] = val;
        }
    }
    
    (sdf, p_width, p_height)
}

fn upload_font_texture(
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

        // Pixel unpack alignment: our data is 1-byte per pixel.
        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);

        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            i32::try_from(glow::R8).expect("R8 fits in i32"),
            i32::try_from(atlas_width).expect("atlas width fits in i32"),
            i32::try_from(atlas_height).expect("atlas height fits in i32"),
            0,
            glow::RED,
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

/// Estimate how wide `text` will render at `text_size`.
#[must_use]
pub fn estimate_text_width(atlas: &FontAtlas, text: &str, text_size: f32) -> f32 {
    let scale = text_size / atlas.rasterize_size;
    let mut width = 0.0f32;
    for c in text.chars() {
        if c == ' ' {
            width = atlas.space_advance.mul_add(scale, width);
        } else if let Some(glyph) = atlas.glyphs.get(&c).or_else(|| atlas.glyphs.get(&'?')) {
            width = glyph.advance_width.mul_add(scale, width);
        }
    }
    width
}
