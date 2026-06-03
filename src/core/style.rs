// Theme Colors (Hex formatted AARRGGBB or RRGGBB)
pub const WINDOW_WIDTH: usize = 420;
pub const WINDOW_HEIGHT: usize = 760;
pub const PANEL_PADDING: f32 = 22.0;
pub const BUTTON_HEIGHT: f32 = 86.0;
pub const BUTTON_GAP: f32 = 16.0;
pub const CARD_RADIUS: f32 = 18.0;
pub const SHADOW_OFFSET_Y: f32 = 7.0;
pub const SHADOW_SPREAD: f32 = 4.0;
pub const ICON_BOX_SIZE: f32 = 56.0;

pub const BACKGROUND: u32 = 0x101820;
pub const BUTTON_TEXT: u32 = 0xF2F5F7;
pub const BUTTON_MUTED: u32 = 0xAFC4CC;
pub const SHADOW: u32 = 0x0B1116;
pub const CARD_SETTINGS: u32 = 0x1B9AAA;
pub const CARD_CONTACTS: u32 = 0x2C8FA3;
pub const CARD_CAMERA: u32 = 0x246E7D;
pub const CARD_SETTINGS_HOVER: u32 = 0x23AFC4;
pub const CARD_CONTACTS_HOVER: u32 = 0x36A3B8;
pub const CARD_CAMERA_HOVER: u32 = 0x2D8091;
pub const ICON_SURFACE: u32 = 0x0F3940;
pub const HEADER_SURFACE: u32 = 0x15232C;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Display {
    #[default]
    Flex,
    None,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum FlexDirection {
    #[default]
    Column,
    Row,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum JustifyContent {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum AlignItems {
    #[default]
    Stretch,
    Start,
    Center,
    End,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Dimension {
    #[default]
    Auto,
    Pixels(f32),
    Percent(f32),
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RectOffset {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl RectOffset {
    pub const fn zero() -> Self {
        Self {
            left: 0.0,
            right: 0.0,
            top: 0.0,
            bottom: 0.0,
        }
    }

    pub const fn all(val: f32) -> Self {
        Self {
            left: val,
            right: val,
            top: val,
            bottom: val,
        }
    }
}

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
    pub fn from_tailwind(classes: &str) -> Self {
        let mut style = Style::default();
        for class in classes.split_whitespace() {
            match class {
                "flex" => style.display = Display::Flex,
                "hidden" => style.display = Display::None,

                "flex-row" | "row" => style.flex_direction = FlexDirection::Row,
                "flex-col" | "col" => style.flex_direction = FlexDirection::Column,

                "justify-start" => style.justify_content = JustifyContent::Start,
                "justify-center" => style.justify_content = JustifyContent::Center,
                "justify-end" => style.justify_content = JustifyContent::End,
                "justify-between" => style.justify_content = JustifyContent::SpaceBetween,

                "items-start" => style.align_items = AlignItems::Start,
                "items-center" => style.align_items = AlignItems::Center,
                "items-end" => style.align_items = AlignItems::End,
                "items-stretch" => style.align_items = AlignItems::Stretch,

                "w-full" => style.width = Dimension::Percent(100.0),
                "h-full" => style.height = Dimension::Percent(100.0),
                "w-auto" => style.width = Dimension::Auto,
                "h-auto" => style.height = Dimension::Auto,

                // Color mapping
                "bg-background" => style.background_color = Some(BACKGROUND),
                "bg-header" => style.background_color = Some(HEADER_SURFACE),
                "bg-settings" => style.background_color = Some(CARD_SETTINGS),
                "bg-contacts" => style.background_color = Some(CARD_CONTACTS),
                "bg-camera" => style.background_color = Some(CARD_CAMERA),
                "bg-settings-hover" => style.background_color = Some(CARD_SETTINGS_HOVER),
                "bg-contacts-hover" => style.background_color = Some(CARD_CONTACTS_HOVER),
                "bg-camera-hover" => style.background_color = Some(CARD_CAMERA_HOVER),
                "bg-icon-surface" => style.background_color = Some(ICON_SURFACE),
                "bg-text" => style.background_color = Some(BUTTON_TEXT),
                "bg-muted" => style.background_color = Some(BUTTON_MUTED),

                "text-main" => style.text_color = Some(BUTTON_TEXT),
                "text-muted" => style.text_color = Some(BUTTON_MUTED),

                // Shadows
                "shadow-card" => {
                    style.shadow_color = Some(SHADOW);
                    style.shadow_offset_y = SHADOW_OFFSET_Y;
                    style.shadow_spread = SHADOW_SPREAD;
                }

                // Border radius
                "rounded" => style.border_radius = 4.0,
                "rounded-md" => style.border_radius = 8.0,
                "rounded-lg" => style.border_radius = CARD_RADIUS, // Matches 18.0
                "rounded-2xl" => style.border_radius = 24.0,
                "rounded-full" => style.border_radius = 9999.0,

                other => {
                    // Custom parser for values like p-4, gap-2, w-[200], h-[100], etc.
                    if let Some(val) = other.strip_prefix("p-") {
                        if let Ok(pixels) = val.parse::<f32>() {
                            style.padding = RectOffset::all(pixels * 4.0); // tailwind p-1 = 4px
                        }
                    } else if let Some(val) = other.strip_prefix("px-") {
                        if let Ok(pixels) = val.parse::<f32>() {
                            style.padding.left = pixels * 4.0;
                            style.padding.right = pixels * 4.0;
                        }
                    } else if let Some(val) = other.strip_prefix("py-") {
                        if let Ok(pixels) = val.parse::<f32>() {
                            style.padding.top = pixels * 4.0;
                            style.padding.bottom = pixels * 4.0;
                        }
                    } else if let Some(val) = other.strip_prefix("m-") {
                        if let Ok(pixels) = val.parse::<f32>() {
                            style.margin = RectOffset::all(pixels * 4.0);
                        }
                    } else if let Some(val) = other.strip_prefix("gap-") {
                        if let Ok(pixels) = val.parse::<f32>() {
                            style.gap = pixels * 4.0;
                        }
                    } else if let Some(val) = other.strip_prefix("w-[") {
                        if let Some(end) = val.strip_suffix(']')
                            && let Ok(pixels) = end.parse::<f32>()
                        {
                            style.width = Dimension::Pixels(pixels);
                        }
                    } else if let Some(val) = other.strip_prefix("h-[") {
                        if let Some(end) = val.strip_suffix(']')
                            && let Ok(pixels) = end.parse::<f32>()
                        {
                            style.height = Dimension::Pixels(pixels);
                        }
                    } else if let Some(val) = other.strip_prefix("text-[")
                        && let Some(end) = val.strip_suffix(']')
                        && let Ok(sz) = end.parse::<f32>()
                    {
                        style.text_size = sz;
                    }
                }
            }
        }
        style
    }
}

#[macro_export]
macro_rules! tw {
    ($classes:expr_2021) => {
        $crate::core::style::Style::from_tailwind($classes)
    };
}
