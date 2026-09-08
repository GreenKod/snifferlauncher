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

/// Wrapping behavior for flex containers.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum FlexWrap {
    #[default]
    NoWrap,
    Wrap,
    WrapReverse,
}

/// Positioning strategy for an element.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum Position {
    #[default]
    Relative,
    Absolute,
}

/// How children are distributed along the main axis.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum JustifyContent {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
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
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub enum Dimension {
    #[default]
    Auto,
    Pixels(f32),
    Percent(f32),
}

impl<'de> Deserialize<'de> for Dimension {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum RawDimension {
            Explicit(ExplicitDimension),
            Num(f32),
            #[allow(dead_code)]
            Str(String),
        }

        #[derive(Deserialize)]
        enum ExplicitDimension {
            Auto,
            Pixels(f32),
            Percent(f32),
        }

        match RawDimension::deserialize(deserializer)? {
            RawDimension::Explicit(ExplicitDimension::Auto) | RawDimension::Str(_) => {
                Ok(Self::Auto)
            }
            RawDimension::Explicit(ExplicitDimension::Pixels(px)) => Ok(Self::Pixels(px)),
            RawDimension::Explicit(ExplicitDimension::Percent(pct)) => Ok(Self::Percent(pct)),
            RawDimension::Num(n) => Ok(Self::Pixels(n)),
        }
    }
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
            Display::Flex => Self::Flex,
            Display::None => Self::None,
        }
    }
}

impl From<FlexDirection> for taffy::style::FlexDirection {
    fn from(fd: FlexDirection) -> Self {
        match fd {
            FlexDirection::Column => Self::Column,
            FlexDirection::Row => Self::Row,
        }
    }
}

impl From<FlexWrap> for taffy::style::FlexWrap {
    fn from(fw: FlexWrap) -> Self {
        match fw {
            FlexWrap::NoWrap => Self::NoWrap,
            FlexWrap::Wrap => Self::Wrap,
            FlexWrap::WrapReverse => Self::WrapReverse,
        }
    }
}

impl From<Position> for taffy::style::Position {
    fn from(p: Position) -> Self {
        match p {
            Position::Relative => Self::Relative,
            Position::Absolute => Self::Absolute,
        }
    }
}

impl From<JustifyContent> for taffy::style::JustifyContent {
    fn from(jc: JustifyContent) -> Self {
        match jc {
            JustifyContent::Start => Self::FlexStart,
            JustifyContent::Center => Self::Center,
            JustifyContent::End => Self::FlexEnd,
            JustifyContent::SpaceBetween => Self::SpaceBetween,
            JustifyContent::SpaceAround => Self::SpaceAround,
            JustifyContent::SpaceEvenly => Self::SpaceEvenly,
        }
    }
}

impl From<AlignItems> for taffy::style::AlignItems {
    fn from(ai: AlignItems) -> Self {
        match ai {
            AlignItems::Stretch => Self::Stretch,
            AlignItems::Start => Self::FlexStart,
            AlignItems::Center => Self::Center,
            AlignItems::End => Self::FlexEnd,
        }
    }
}

impl From<Dimension> for taffy::style::Dimension {
    fn from(d: Dimension) -> Self {
        match d {
            Dimension::Auto => Self::Auto,
            Dimension::Pixels(px) => Self::Length(px),
            Dimension::Percent(pct) => Self::Percent(pct / 100.0),
        }
    }
}

impl From<Dimension> for taffy::style::LengthPercentageAuto {
    fn from(d: Dimension) -> Self {
        match d {
            Dimension::Auto => Self::Auto,
            Dimension::Pixels(px) => Self::Length(px),
            Dimension::Percent(pct) => Self::Percent(pct / 100.0),
        }
    }
}

impl From<Dimension> for taffy::style::LengthPercentage {
    fn from(d: Dimension) -> Self {
        match d {
            Dimension::Auto => Self::Length(0.0), // Fallback
            Dimension::Pixels(px) => Self::Length(px),
            Dimension::Percent(pct) => Self::Percent(pct / 100.0),
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
