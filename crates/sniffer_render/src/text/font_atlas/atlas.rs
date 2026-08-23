use super::discovery::discover_system_fonts;
use super::rasterizer::{GlyphData, render_character};
use super::upload::{update_font_texture_sub, upload_font_texture};
use crate::dev_err;
use obfstr::obfstr;
use std::collections::HashMap;
use swash::FontRef;

const PADDING: u32 = 4;
const DEFAULT_ATLAS_DIM: u32 = 1024;

#[derive(Clone, Debug)]
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
    /// True if this glyph contains full RGBA color (e.g. Color Emojis)
    pub is_color: bool,
}

pub struct FontAtlas {
    pub texture: glow::Texture,
    pub glyphs: HashMap<char, GlyphInfo>,
    pub atlas_width: i32,
    pub atlas_height: i32,
    /// The pixel size used when rasterizing glyphs
    pub rasterize_size: f32,
    /// Ascent from `horizontal_line_metrics` (baseline to top of line).
    pub ascent: f32,
    /// Space character advance
    pub space_advance: f32,
    /// Loaded font payloads for multi-font fallback
    pub font_bytes_list: Vec<Vec<u8>>,
    atlas_pixels: Vec<u8>,
    shelf_x: u32,
    shelf_y: u32,
    shelf_height: u32,
}

pub fn build_font_atlas(
    gl: &glow::Context,
    font_bytes: &[u8],
    pixel_size: f32,
) -> Result<FontAtlas, String> {
    if font_bytes.is_empty() {
        return Err(obfstr!("Empty font payload provided to font atlas builder").to_string());
    }

    let font = FontRef::from_index(font_bytes, 0)
        .ok_or_else(|| obfstr!("Failed to parse primary font payload").to_string())?;

    let metrics = font.metrics(&[]);
    let units_per_em = f32::from(metrics.units_per_em);
    let scale_factor = pixel_size / units_per_em;
    let ascent = metrics.ascent * scale_factor;

    let space_glyph_id = font.charmap().map(' ');
    let space_advance = if space_glyph_id != 0 {
        font.glyph_metrics(&[]).advance_width(space_glyph_id) * scale_factor
    } else {
        pixel_size * 0.3
    };

    let mut font_bytes_list = vec![font_bytes.to_vec()];
    let system_fonts = discover_system_fonts();
    font_bytes_list.extend(system_fonts);

    let atlas_w = DEFAULT_ATLAS_DIM;
    let atlas_h = DEFAULT_ATLAS_DIM;
    let atlas_pixels = vec![0u8; (atlas_w * atlas_h * 4) as usize];

    let font_texture = upload_font_texture(gl, atlas_w, atlas_h, &atlas_pixels)?;

    let mut atlas = FontAtlas {
        texture: font_texture,
        glyphs: HashMap::with_capacity(256),
        atlas_width: i32::try_from(atlas_w).expect("atlas width fits in i32"),
        atlas_height: i32::try_from(atlas_h).expect("atlas height fits in i32"),
        rasterize_size: pixel_size,
        ascent,
        space_advance,
        font_bytes_list,
        atlas_pixels,
        shelf_x: PADDING,
        shelf_y: PADDING,
        shelf_height: 0,
    };

    // Preload basic ASCII characters and common letters
    let mut initial_chars = String::with_capacity(128);
    for c in 32u8..128u8 {
        initial_chars.push(char::from(c));
    }
    initial_chars.push_str("çÇğĞıİöÖşŞüÜ");
    atlas.ensure_glyphs(gl, &initial_chars);

    dev_err!(
        "{} {}x{} {}, {} {}, {}={:.1}, {} fonts in fallback stack",
        obfstr!("[FontAtlas] Built dynamic 2D"),
        atlas_w,
        atlas_h,
        obfstr!("atlas,"),
        atlas.glyphs.len(),
        obfstr!("glyphs,"),
        obfstr!("ascent"),
        ascent,
        atlas.font_bytes_list.len()
    );

    Ok(atlas)
}

impl FontAtlas {
    /// Constructs a mock FontAtlas instance for headless tests and text measurement.
    #[must_use]
    pub fn new_mock(
        texture: glow::Texture,
        glyphs: HashMap<char, GlyphInfo>,
        rasterize_size: f32,
        ascent: f32,
        space_advance: f32,
    ) -> Self {
        Self {
            texture,
            glyphs,
            atlas_width: 1024,
            atlas_height: 1024,
            rasterize_size,
            ascent,
            space_advance,
            font_bytes_list: Vec::new(),
            atlas_pixels: Vec::new(),
            shelf_x: PADDING,
            shelf_y: PADDING,
            shelf_height: 0,
        }
    }

    /// Ensures that all glyphs in `text` are rasterized and uploaded to the GPU texture atlas.
    pub fn ensure_glyphs(&mut self, gl: &glow::Context, text: &str) {
        for c in text.chars() {
            if c == ' ' || self.glyphs.contains_key(&c) {
                continue;
            }
            self.ensure_glyph(gl, c);
        }
    }

    pub fn ensure_glyph(&mut self, gl: &glow::Context, character: char) -> Option<&GlyphInfo> {
        if self.glyphs.contains_key(&character) {
            return self.glyphs.get(&character);
        }

        let glyph_data = render_character(character, &self.font_bytes_list, self.rasterize_size);
        let data = match glyph_data {
            Some(d) => d,
            None => {
                if character != '?' {
                    if let Some(fallback) = self.ensure_glyph(gl, '?').cloned() {
                        self.glyphs.insert(character, fallback);
                        return self.glyphs.get(&character);
                    }
                }
                GlyphData {
                    width: 0,
                    height: 0,
                    bearing_x: 0.0,
                    bearing_y: 0.0,
                    advance_width: self.rasterize_size * 0.5,
                    is_color: false,
                    bitmap: Vec::new(),
                }
            }
        };

        let gw = data.width;
        let gh = data.height;

        if gw == 0 || gh == 0 {
            self.glyphs.insert(
                character,
                GlyphInfo {
                    atlas_x: 0,
                    atlas_y: 0,
                    width: 0,
                    height: 0,
                    bearing_x: data.bearing_x,
                    bearing_y: data.bearing_y,
                    advance_width: data.advance_width,
                    is_color: data.is_color,
                },
            );
            return self.glyphs.get(&character);
        }

        let atlas_w = u32::try_from(self.atlas_width).unwrap_or(1024);
        let atlas_h = u32::try_from(self.atlas_height).unwrap_or(1024);

        if self.shelf_x + gw + PADDING > atlas_w {
            self.shelf_y += self.shelf_height + PADDING;
            self.shelf_x = PADDING;
            self.shelf_height = 0;
        }

        if self.shelf_y + gh + PADDING > atlas_h {
            dev_err!(
                "[FontAtlas] Warning: Atlas texture is full, cannot pack '{}'",
                character
            );
            return None;
        }

        let dst_x = self.shelf_x;
        let dst_y = self.shelf_y;

        // Copy glyph pixels into the RGBA atlas
        for row in 0..gh {
            for col in 0..gw {
                let dx = dst_x + col;
                let dy = dst_y + row;
                let dst_idx = ((dy * atlas_w + dx) * 4) as usize;

                if data.is_color {
                    let src_idx = ((row * gw + col) * 4) as usize;
                    if src_idx + 4 <= data.bitmap.len() && dst_idx + 4 <= self.atlas_pixels.len() {
                        self.atlas_pixels[dst_idx..dst_idx + 4]
                            .copy_from_slice(&data.bitmap[src_idx..src_idx + 4]);
                    }
                } else {
                    let src_idx = (row * gw + col) as usize;
                    if src_idx < data.bitmap.len() && dst_idx + 4 <= self.atlas_pixels.len() {
                        let val = data.bitmap[src_idx];
                        self.atlas_pixels[dst_idx] = val;
                        self.atlas_pixels[dst_idx + 1] = val;
                        self.atlas_pixels[dst_idx + 2] = val;
                        self.atlas_pixels[dst_idx + 3] = val;
                    }
                }
            }
        }

        // Upload sub-rectangle to GPU
        update_font_texture_sub(
            gl,
            self.texture,
            atlas_w,
            dst_x,
            dst_y,
            gw,
            gh,
            &self.atlas_pixels,
        );

        self.shelf_x += gw + PADDING;
        self.shelf_height = self.shelf_height.max(gh);

        self.glyphs.insert(
            character,
            GlyphInfo {
                atlas_x: dst_x,
                atlas_y: dst_y,
                width: gw,
                height: gh,
                bearing_x: data.bearing_x,
                bearing_y: data.bearing_y,
                advance_width: data.advance_width,
                is_color: data.is_color,
            },
        );

        self.glyphs.get(&character)
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
        } else if let Some(glyph) = atlas.glyphs.get(&c) {
            width = glyph.advance_width.mul_add(scale, width);
        } else {
            let mut found = false;
            for font_bytes in &atlas.font_bytes_list {
                if let Some(font) = FontRef::from_index(font_bytes, 0) {
                    let glyph_id = font.charmap().map(c);
                    if glyph_id != 0 {
                        let glyph_metrics = font.glyph_metrics(&[]);
                        let units_per_em = f32::from(font.metrics(&[]).units_per_em);
                        let sf = atlas.rasterize_size / units_per_em;
                        let adv = glyph_metrics.advance_width(glyph_id) * sf;
                        width = adv.mul_add(scale, width);
                        found = true;
                        break;
                    }
                }
            }
            if !found {
                if let Some(fallback) = atlas.glyphs.get(&'?') {
                    width = fallback.advance_width.mul_add(scale, width);
                } else {
                    width += text_size * 0.6;
                }
            }
        }
    }
    width
}
