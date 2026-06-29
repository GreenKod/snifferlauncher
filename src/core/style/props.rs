use super::layout::{AlignItems, Dimension, Display, FlexDirection, JustifyContent, RectOffset};

/// All visual and layout properties of a single UI element.
#[derive(Clone, Debug, PartialEq)]
pub struct Style {
    pub display: Display,
    pub flex_direction: FlexDirection,
    pub justify_content: JustifyContent,
    pub align_items: AlignItems,

    pub width: Dimension,
    pub height: Dimension,

    pub padding: RectOffset,
    pub margin: RectOffset,
    pub gap: f32,

    pub background_color: Option<u32>,
    pub border_radius: f32,
    pub border_color: Option<u32>,
    pub border_width: f32,

    pub shadow_color: Option<u32>,
    pub shadow_offset_y: f32,
    pub shadow_spread: f32,

    pub text_color: Option<u32>,
    pub text_size: f32,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Start,
            align_items: AlignItems::Stretch,
            width: Dimension::Auto,
            height: Dimension::Auto,
            padding: RectOffset::zero(),
            margin: RectOffset::zero(),
            gap: 0.0,
            background_color: None,
            border_radius: 0.0,
            border_color: None,
            border_width: 0.0,
            shadow_color: None,
            shadow_offset_y: 0.0,
            shadow_spread: 0.0,
            text_color: None,
            text_size: 16.0,
        }
    }
}

impl Style {
    /// Start building a `Style` with pre-scaled physical pixel values.
    /// Use this when you already have values in physical pixels
    /// (e.g. from `ScreenMetrics::dp()`).
    #[must_use]
    pub fn builder() -> StyleBuilder {
        StyleBuilder::new()
    }
}

// ── StyleBuilder ──────────────────────────────────────────────────────────────

/// Fluent builder for `Style` — accepts physical-pixel values directly.
#[derive(Default)]
pub struct StyleBuilder(Style);

impl StyleBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self(Style::default())
    }

    #[must_use]
    pub const fn background_color(mut self, color: u32) -> Self {
        self.0.background_color = Some(color);
        self
    }

    /// Set background colour from a tailwind-style semantic name
    /// (e.g. `"bg-settings-hover"`). Falls back to no colour on unknown names.
    #[must_use]
    pub fn background_color_from_tailwind(mut self, name: &str) -> Self {
        use super::color::{
            BACKGROUND, CARD_CAMERA, CARD_CAMERA_HOVER, CARD_CONTACTS, CARD_CONTACTS_HOVER,
            CARD_SETTINGS, CARD_SETTINGS_HOVER, HEADER_SURFACE,
        };
        let color = match name {
            "bg-background" => Some(BACKGROUND),
            "bg-header" => Some(HEADER_SURFACE),
            "bg-settings" => Some(CARD_SETTINGS),
            "bg-contacts" => Some(CARD_CONTACTS),
            "bg-camera" => Some(CARD_CAMERA),
            "bg-settings-hover" => Some(CARD_SETTINGS_HOVER),
            "bg-contacts-hover" => Some(CARD_CONTACTS_HOVER),
            "bg-camera-hover" => Some(CARD_CAMERA_HOVER),
            _ => None,
        };
        self.0.background_color = color;
        self
    }

    #[must_use]
    pub const fn border_radius(mut self, r: f32) -> Self {
        self.0.border_radius = r;
        self
    }

    #[must_use]
    pub const fn padding_all(mut self, v: f32) -> Self {
        self.0.padding = RectOffset::all(v);
        self
    }

    #[must_use]
    pub const fn padding_horizontal(mut self, v: f32) -> Self {
        self.0.padding.left = v;
        self.0.padding.right = v;
        self
    }

    #[must_use]
    pub const fn padding_vertical(mut self, v: f32) -> Self {
        self.0.padding.top = v;
        self.0.padding.bottom = v;
        self
    }

    #[must_use]
    pub const fn gap(mut self, v: f32) -> Self {
        self.0.gap = v;
        self
    }

    #[must_use]
    pub const fn column(mut self) -> Self {
        self.0.flex_direction = FlexDirection::Column;
        self
    }

    #[must_use]
    pub const fn row(mut self) -> Self {
        self.0.flex_direction = FlexDirection::Row;
        self
    }

    #[must_use]
    pub const fn align_stretch(mut self) -> Self {
        self.0.align_items = AlignItems::Stretch;
        self
    }

    #[must_use]
    pub const fn align_start(mut self) -> Self {
        self.0.align_items = AlignItems::Start;
        self
    }

    #[must_use]
    pub const fn align_center(mut self) -> Self {
        self.0.align_items = AlignItems::Center;
        self
    }

    #[must_use]
    pub const fn width_percent(mut self, p: f32) -> Self {
        self.0.width = Dimension::Percent(p);
        self
    }

    #[must_use]
    pub const fn height_percent(mut self, p: f32) -> Self {
        self.0.height = Dimension::Percent(p);
        self
    }

    #[must_use]
    pub const fn width_pixels(mut self, px: f32) -> Self {
        self.0.width = Dimension::Pixels(px);
        self
    }

    #[must_use]
    pub const fn height_pixels(mut self, px: f32) -> Self {
        self.0.height = Dimension::Pixels(px);
        self
    }

    #[must_use]
    pub const fn text_color(mut self, color: u32) -> Self {
        self.0.text_color = Some(color);
        self
    }

    #[must_use]
    pub const fn text_size(mut self, size: f32) -> Self {
        self.0.text_size = size;
        self
    }

    /// Apply the standard card drop-shadow (scaled values from `color.rs`).
    #[must_use]
    pub const fn shadow_card(mut self) -> Self {
        use super::color::{SHADOW, SHADOW_OFFSET_Y, SHADOW_SPREAD};
        self.0.shadow_color = Some(SHADOW);
        self.0.shadow_offset_y = SHADOW_OFFSET_Y;
        self.0.shadow_spread = SHADOW_SPREAD;
        self
    }

    #[must_use]
    pub const fn build(self) -> Style {
        self.0
    }
}
