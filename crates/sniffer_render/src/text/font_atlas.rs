#![allow(clippy::pedantic, clippy::nursery)]

use crate::dev_err;
use glow::HasContext;
use obfstr::obfstr;
use swash::FontRef;
use swash::scale::{Render, ScaleContext, Source, StrikeWith, image::Content};

const PADDING: u32 = 4;
const DEFAULT_ATLAS_DIM: u32 = 1024;

struct GlyphData {
    width: u32,
    height: u32,
    bearing_x: f32,
    bearing_y: f32,
    advance_width: f32,
    is_color: bool,
    bitmap: Vec<u8>,
}

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
    pub glyphs: std::collections::HashMap<char, GlyphInfo>,
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

/// Discovers available system fonts and color emoji fonts on the host platform.
#[must_use]
pub fn discover_system_fonts() -> Vec<Vec<u8>> {
    let mut font_data_list = Vec::new();

    #[cfg(target_os = "android")]
    let candidate_paths = [
        "/system/fonts/Roboto-Regular.ttf",
        "/system/fonts/NotoSans-Regular.ttf",
        "/system/fonts/NotoSansCJK-Regular.ttc",
        "/system/fonts/NotoColorEmoji.ttf",
        "/system/fonts/GoogleColorEmoji.ttf",
    ];

    #[cfg(target_os = "windows")]
    let candidate_paths = [
        "C:\\Windows\\Fonts\\segoeui.ttf",
        "C:\\Windows\\Fonts\\seguiemj.ttf",
        "C:\\Windows\\Fonts\\arial.ttf",
    ];

    #[cfg(not(any(target_os = "android", target_os = "windows")))]
    let candidate_paths = [
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/noto/NotoColorEmoji.ttf",
        "/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf",
    ];

    for path in candidate_paths {
        if let Ok(bytes) = std::fs::read(path) {
            if !bytes.is_empty() {
                font_data_list.push(bytes);
            }
        }
    }

    font_data_list
}

/// Build a dynamic font atlas texture and metrics from a TTF payload with system font fallbacks.
///
/// # Errors
///
/// Returns an error if no font can be parsed or the GPU texture upload fails.
pub fn build_font_atlas(
    gl: &glow::Context,
    font_bytes: &[u8],
    pixel_size: f32,
) -> Result<FontAtlas, String> {
    let primary_font =
        FontRef::from_index(font_bytes, 0).ok_or_else(|| "Failed to parse font".to_string())?;

    let font_metrics = primary_font.metrics(&[]);
    let units_per_em = f32::from(font_metrics.units_per_em);
    let scale_factor = pixel_size / units_per_em;
    let ascent = font_metrics.ascent * scale_factor;

    let space_advance = {
        let charmap = primary_font.charmap();
        let glyph_id = charmap.map(' ');
        let glyph_metrics = primary_font.glyph_metrics(&[]);
        glyph_metrics.advance_width(glyph_id) * scale_factor
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
        glyphs: std::collections::HashMap::with_capacity(256),
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
        glyphs: std::collections::HashMap<char, GlyphInfo>,
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
            self.rasterize_and_pack_glyph(gl, c);
        }
    }

    fn rasterize_and_pack_glyph(&mut self, gl: &glow::Context, character: char) {
        if let Some(glyph_data) = self.render_character(character) {
            let gw = glyph_data.width;
            let gh = glyph_data.height;

            if gw == 0 || gh == 0 {
                self.glyphs.insert(
                    character,
                    GlyphInfo {
                        atlas_x: 0,
                        atlas_y: 0,
                        width: 0,
                        height: 0,
                        bearing_x: glyph_data.bearing_x,
                        bearing_y: glyph_data.bearing_y,
                        advance_width: glyph_data.advance_width,
                        is_color: glyph_data.is_color,
                    },
                );
                return;
            }

            let atlas_w = self.atlas_width.cast_unsigned();
            let atlas_h = self.atlas_height.cast_unsigned();

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
                return;
            }

            let dst_x = self.shelf_x;
            let dst_y = self.shelf_y;

            // Copy glyph pixels into the RGBA atlas
            for row in 0..gh {
                for col in 0..gw {
                    let dx = dst_x + col;
                    let dy = dst_y + row;
                    let dst_idx = ((dy * atlas_w + dx) * 4) as usize;

                    if glyph_data.is_color {
                        let src_idx = ((row * gw + col) * 4) as usize;
                        if src_idx + 4 <= glyph_data.bitmap.len()
                            && dst_idx + 4 <= self.atlas_pixels.len()
                        {
                            self.atlas_pixels[dst_idx..dst_idx + 4]
                                .copy_from_slice(&glyph_data.bitmap[src_idx..src_idx + 4]);
                        }
                    } else {
                        let src_idx = (row * gw + col) as usize;
                        if src_idx < glyph_data.bitmap.len()
                            && dst_idx + 4 <= self.atlas_pixels.len()
                        {
                            let val = glyph_data.bitmap[src_idx];
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

            self.glyphs.insert(
                character,
                GlyphInfo {
                    atlas_x: dst_x,
                    atlas_y: dst_y,
                    width: gw,
                    height: gh,
                    bearing_x: glyph_data.bearing_x,
                    bearing_y: glyph_data.bearing_y,
                    advance_width: glyph_data.advance_width,
                    is_color: glyph_data.is_color,
                },
            );

            self.shelf_x += gw + PADDING;
            self.shelf_height = self.shelf_height.max(gh);
        }
    }

    fn render_character(&self, character: char) -> Option<GlyphData> {
        let pixel_size = self.rasterize_size;

        for font_bytes in &self.font_bytes_list {
            if let Some(font) = FontRef::from_index(font_bytes, 0) {
                let charmap = font.charmap();
                let glyph_id = charmap.map(character);
                if glyph_id == 0 {
                    continue;
                }

                let font_metrics = font.metrics(&[]);
                let units_per_em = f32::from(font_metrics.units_per_em);
                let scale_factor = pixel_size / units_per_em;
                let glyph_metrics = font.glyph_metrics(&[]);
                let advance_width = glyph_metrics.advance_width(glyph_id) * scale_factor;

                let mut context = ScaleContext::new();
                let mut scaler = context.builder(font).size(pixel_size).hint(true).build();

                let renderer = Render::new(&[
                    Source::ColorBitmap(StrikeWith::BestFit),
                    Source::ColorOutline(0),
                    Source::Outline,
                    Source::Bitmap(StrikeWith::BestFit),
                ]);

                let image = renderer.render(&mut scaler, glyph_id);
                if let Some(img) = image {
                    let w = img.placement.width;
                    let h = img.placement.height;
                    #[allow(clippy::cast_precision_loss)]
                    let bx = img.placement.left as f32;
                    #[allow(clippy::cast_precision_loss)]
                    let by = img.placement.top as f32;

                    match img.content {
                        Content::Color => {
                            return Some(GlyphData {
                                width: w,
                                height: h,
                                bearing_x: bx,
                                bearing_y: by,
                                advance_width,
                                is_color: true,
                                bitmap: img.data,
                            });
                        }
                        Content::SubpixelMask => {
                            let mut gray = Vec::with_capacity(img.data.len() / 3);
                            for chunk in img.data.as_chunks::<3>().0 {
                                let g = (u32::from(chunk[0])
                                    + u32::from(chunk[1])
                                    + u32::from(chunk[2]))
                                    / 3;
                                #[allow(clippy::cast_possible_truncation)]
                                gray.push(g as u8);
                            }
                            let spread = 8;
                            let (sdf_bitmap, sdf_w, sdf_h) = generate_sdf(&gray, w, h, spread);
                            return Some(GlyphData {
                                width: sdf_w,
                                height: sdf_h,
                                bearing_x: bx - spread as f32,
                                bearing_y: by + spread as f32,
                                advance_width,
                                is_color: false,
                                bitmap: sdf_bitmap,
                            });
                        }
                        Content::Mask => {
                            let spread = 8;
                            let (sdf_bitmap, sdf_w, sdf_h) = generate_sdf(&img.data, w, h, spread);
                            return Some(GlyphData {
                                width: sdf_w,
                                height: sdf_h,
                                bearing_x: bx - spread as f32,
                                bearing_y: by + spread as f32,
                                advance_width,
                                is_color: false,
                                bitmap: sdf_bitmap,
                            });
                        }
                    }
                }
            }
        }

        // Fallback for completely unresolvable glyphs
        None
    }
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

fn update_font_texture_sub(
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
            // Find advance width from fallback fonts directly
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
