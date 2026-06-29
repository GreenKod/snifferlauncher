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
