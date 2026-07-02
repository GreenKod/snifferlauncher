use serde::{Deserialize, Serialize};

/// Controls whether an element participates in layout.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum Display {
    #[default]
    Flex,
    None,
}

/// Main axis direction for a flex container.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum FlexDirection {
    #[default]
    Column,
    Row,
}

/// How children are distributed along the main axis.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum JustifyContent {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
}

/// How children are aligned along the cross axis.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum AlignItems {
    #[default]
    Stretch,
    Start,
    Center,
    End,
}

/// A size value — either automatic, a fixed pixel count, or a percentage.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum Dimension {
    #[default]
    Auto,
    Pixels(f32),
    Percent(f32),
}

/// How an image should be resized to fit its container.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum ObjectFit {
    #[default]
    Fill,
    Contain,
    Cover,
}

/// Uniform or per-side offsets used for padding and margin.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RectOffset {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl RectOffset {
    #[must_use]
    pub const fn zero() -> Self {
        Self {
            left: 0.0,
            right: 0.0,
            top: 0.0,
            bottom: 0.0,
        }
    }

    #[must_use]
    pub const fn all(val: f32) -> Self {
        Self {
            left: val,
            right: val,
            top: val,
            bottom: val,
        }
    }
}

impl From<Display> for taffy::style::Display {
    fn from(d: Display) -> Self {
        match d {
            Display::Flex => taffy::style::Display::Flex,
            Display::None => taffy::style::Display::None,
        }
    }
}

impl From<FlexDirection> for taffy::style::FlexDirection {
    fn from(fd: FlexDirection) -> Self {
        match fd {
            FlexDirection::Column => taffy::style::FlexDirection::Column,
            FlexDirection::Row => taffy::style::FlexDirection::Row,
        }
    }
}

impl From<JustifyContent> for taffy::style::JustifyContent {
    fn from(jc: JustifyContent) -> Self {
        match jc {
            JustifyContent::Start => taffy::style::JustifyContent::FlexStart,
            JustifyContent::Center => taffy::style::JustifyContent::Center,
            JustifyContent::End => taffy::style::JustifyContent::FlexEnd,
            JustifyContent::SpaceBetween => taffy::style::JustifyContent::SpaceBetween,
        }
    }
}

impl From<AlignItems> for taffy::style::AlignItems {
    fn from(ai: AlignItems) -> Self {
        match ai {
            AlignItems::Stretch => taffy::style::AlignItems::Stretch,
            AlignItems::Start => taffy::style::AlignItems::FlexStart,
            AlignItems::Center => taffy::style::AlignItems::Center,
            AlignItems::End => taffy::style::AlignItems::FlexEnd,
        }
    }
}

impl From<Dimension> for taffy::style::Dimension {
    fn from(d: Dimension) -> Self {
        match d {
            Dimension::Auto => taffy::style::Dimension::Auto,
            Dimension::Pixels(px) => taffy::style::Dimension::Length(px),
            Dimension::Percent(pct) => taffy::style::Dimension::Percent(pct / 100.0),
        }
    }
}

impl From<Dimension> for taffy::style::LengthPercentageAuto {
    fn from(d: Dimension) -> Self {
        match d {
            Dimension::Auto => taffy::style::LengthPercentageAuto::Auto,
            Dimension::Pixels(px) => taffy::style::LengthPercentageAuto::Length(px),
            Dimension::Percent(pct) => taffy::style::LengthPercentageAuto::Percent(pct / 100.0),
        }
    }
}

impl From<Dimension> for taffy::style::LengthPercentage {
    fn from(d: Dimension) -> Self {
        match d {
            Dimension::Auto => taffy::style::LengthPercentage::Length(0.0), // Fallback
            Dimension::Pixels(px) => taffy::style::LengthPercentage::Length(px),
            Dimension::Percent(pct) => taffy::style::LengthPercentage::Percent(pct / 100.0),
        }
    }
}

impl From<RectOffset> for taffy::geometry::Rect<taffy::style::LengthPercentage> {
    fn from(r: RectOffset) -> Self {
        Self {
            left: taffy::style::LengthPercentage::Length(r.left),
            right: taffy::style::LengthPercentage::Length(r.right),
            top: taffy::style::LengthPercentage::Length(r.top),
            bottom: taffy::style::LengthPercentage::Length(r.bottom),
        }
    }
}

impl From<RectOffset> for taffy::geometry::Rect<taffy::style::LengthPercentageAuto> {
    fn from(r: RectOffset) -> Self {
        Self {
            left: taffy::style::LengthPercentageAuto::Length(r.left),
            right: taffy::style::LengthPercentageAuto::Length(r.right),
            top: taffy::style::LengthPercentageAuto::Length(r.top),
            bottom: taffy::style::LengthPercentageAuto::Length(r.bottom),
        }
    }
}
