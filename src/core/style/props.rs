use super::layout::{
    AlignItems, Dimension, Display, FlexDirection, JustifyContent, ObjectFit, RectOffset,
};
use serde::{Deserialize, Serialize};

/// All visual and layout properties of a single UI element.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
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

    pub overflow_hidden: bool,
    pub object_fit: ObjectFit,
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
            overflow_hidden: false,
            object_fit: ObjectFit::Fill,
        }
    }
}

impl Style {
    #[must_use]
    pub fn builder() -> StyleBuilder {
        StyleBuilder::new()
    }
}

#[derive(Default)]
pub struct StyleBuilder(Style);

impl StyleBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self(Style::default())
    }
    #[must_use]
    pub const fn build(self) -> Style {
        self.0
    }
}

#[derive(Clone, Default, Debug)]
pub struct StyleOverride {
    pub background_color: Option<u32>,
    pub text_color: Option<u32>,
    pub border_color: Option<u32>,
}

impl StyleOverride {
    #[must_use]
    pub const fn apply(&self, mut base: Style) -> Style {
        if let Some(c) = self.background_color {
            base.background_color = Some(c);
        }
        if let Some(c) = self.text_color {
            base.text_color = Some(c);
        }
        if let Some(c) = self.border_color {
            base.border_color = Some(c);
        }
        base
    }
}
