use swash::FontRef;
use swash::scale::{Render, ScaleContext, Source, StrikeWith, image::Content};

pub struct GlyphData {
    pub width: u32,
    pub height: u32,
    pub bearing_x: f32,
    pub bearing_y: f32,
    pub advance_width: f32,
    pub is_color: bool,
    pub bitmap: Vec<u8>,
}

pub fn render_character(
    character: char,
    font_bytes_list: &[Vec<u8>],
    pixel_size: f32,
) -> Option<GlyphData> {
    for font_bytes in font_bytes_list {
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
                            let g =
                                (u32::from(chunk[0]) + u32::from(chunk[1]) + u32::from(chunk[2]))
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

    None
}

pub fn generate_sdf(bitmap: &[u8], width: u32, height: u32, spread: u32) -> (Vec<u8>, u32, u32) {
    if width == 0 || height == 0 {
        return (Vec::new(), 0, 0);
    }

    let p_width = width + 2 * spread;
    let p_height = height + 2 * spread;
    let mut padded = vec![0u8; (p_width * p_height) as usize];

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

            let norm = 0.5 + 0.5 * (dist / spread_f);

            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let val = (norm * 255.0).clamp(0.0, 255.0) as u8;
            sdf[idx] = val;
        }
    }

    (sdf, p_width, p_height)
}
