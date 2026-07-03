use super::layout::{
    AlignItems, Dimension, Display, FlexDirection, FlexWrap, JustifyContent, ObjectFit, Position, RectOffset,
};
use serde::{Deserialize, Serialize};

/// All visual and layout properties of a single UI element.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Style {
    pub display: Display,
    pub position: Position,
    pub top: Dimension,
    pub right: Dimension,
    pub bottom: Dimension,
    pub left: Dimension,

    pub flex_direction: FlexDirection,
    pub flex_wrap: FlexWrap,
    pub justify_content: JustifyContent,
    pub align_items: AlignItems,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Dimension,

    pub width: Dimension,
    pub height: Dimension,
    pub min_width: Dimension,
    pub max_width: Dimension,
    pub min_height: Dimension,
    pub max_height: Dimension,

    pub padding: RectOffset,
    pub margin: RectOffset,
    pub gap: f32,

    pub opacity: f32,
    pub background_color: Option<u32>,
    pub background_gradient: Option<(u32, u32)>,
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
            position: Position::Relative,
            top: Dimension::Auto,
            right: Dimension::Auto,
            bottom: Dimension::Auto,
            left: Dimension::Auto,
            flex_direction: FlexDirection::Column,
            flex_wrap: FlexWrap::NoWrap,
            justify_content: JustifyContent::Start,
            align_items: AlignItems::Stretch,
            flex_grow: 0.0,
            flex_shrink: 0.0,
            flex_basis: Dimension::Auto,
            width: Dimension::Auto,
            height: Dimension::Auto,
            min_width: Dimension::Auto,
            max_width: Dimension::Auto,
            min_height: Dimension::Auto,
            max_height: Dimension::Auto,
            padding: RectOffset::zero(),
            margin: RectOffset::zero(),
            gap: 0.0,
            opacity: 1.0,
            background_color: None,
            background_gradient: None,
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
