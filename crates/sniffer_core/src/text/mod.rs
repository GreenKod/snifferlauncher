use std::collections::HashMap;
use std::sync::RwLock;
use swash::FontRef;

pub const DEFAULT_FONT: &[u8] = include_bytes!("../../fonts/audiowide.ttf");

#[derive(Clone)]
struct CachedMeasure {
    text: String,
    size_scaled: u32,
    dims: (f32, f32),
}

pub struct TextMeasurer<'a> {
    font: FontRef<'a>,
    cache: RwLock<HashMap<u64, CachedMeasure>>,
}

impl<'a> TextMeasurer<'a> {
    #[must_use]
    pub fn new(font_bytes: &'a [u8]) -> Option<Self> {
        let font = FontRef::from_index(font_bytes, 0)?;
        Some(Self {
            font,
            cache: RwLock::new(HashMap::new()),
        })
    }

    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn measure(&self, text: &str, text_size: f32) -> (f32, f32) {
        let size_scaled = (text_size * 100.0).max(0.0).round() as u32;
        let hash = crate::ui::widget::fnv1a(text.as_bytes()) ^ (u64::from(size_scaled) << 32);

        if let Ok(cache) = self.cache.read()
            && let Some(entry) = cache.get(&hash)
            && entry.size_scaled == size_scaled
            && entry.text == text
        {
            return entry.dims;
        }

        let metrics = self.font.metrics(&[]);
        let units_per_em = f32::from(metrics.units_per_em);
        let scale_factor = text_size / units_per_em;

        let mut width = 0.0;
        let charmap = self.font.charmap();
        let glyph_metrics = self.font.glyph_metrics(&[]);

        for c in text.chars() {
            let glyph_id = charmap.map(c);
            let advance = if glyph_id == 0 {
                units_per_em * 0.8
            } else {
                glyph_metrics.advance_width(glyph_id)
            };
            width = advance.mul_add(scale_factor, width);
        }

        // Use text_size as a baseline, or use real ascent+descent.
        // For UI, usually text_size or slightly larger is preferred.
        let height = text_size * 1.2;
        let dims = (width, height);

        if let Ok(mut cache) = self.cache.write() {
            if cache.len() >= 1024 {
                cache.clear();
            }
            cache.insert(
                hash,
                CachedMeasure {
                    text: text.to_string(),
                    size_scaled,
                    dims,
                },
            );
        }

        dims
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_measurer_caching() {
        let measurer = TextMeasurer::new(DEFAULT_FONT).expect("valid font");
        let (w1, h1) = measurer.measure("SnifferLauncher", 16.0);
        assert!(w1 > 0.0);
        assert!(h1 > 0.0);

        // Second measure should hit cache and yield identical results
        let (w2, h2) = measurer.measure("SnifferLauncher", 16.0);
        assert_eq!(w1, w2);
        assert_eq!(h1, h2);

        // Different size should yield different dimensions
        let (w3, h3) = measurer.measure("SnifferLauncher", 32.0);
        assert!(w3 > w1);
        assert!(h3 > h1);
    }
}
