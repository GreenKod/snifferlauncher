use swash::FontRef;

pub const DEFAULT_FONT: &[u8] = include_bytes!("../fonts/audiowide.ttf");

pub struct TextMeasurer<'a> {
    font: FontRef<'a>,
}

impl<'a> TextMeasurer<'a> {
    #[must_use]
    pub fn new(font_bytes: &'a [u8]) -> Option<Self> {
        let font = FontRef::from_index(font_bytes, 0)?;
        Some(Self { font })
    }

    #[must_use]
    pub fn measure(&self, text: &str, text_size: f32) -> (f32, f32) {
        let metrics = self.font.metrics(&[]);
        let units_per_em = f32::from(metrics.units_per_em);
        let scale_factor = text_size / units_per_em;

        let mut width = 0.0;
        let charmap = self.font.charmap();
        let glyph_metrics = self.font.glyph_metrics(&[]);

        for c in text.chars() {
            let glyph_id = charmap.map(c);
            let advance = glyph_metrics.advance_width(glyph_id);
            width = advance.mul_add(scale_factor, width);
        }

        // Use text_size as a baseline, or use real ascent+descent.
        // For UI, usually text_size or slightly larger is preferred.
        let height = text_size * 1.2;

        (width, height)
    }
}

