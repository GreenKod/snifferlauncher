use super::layout::{AlignItems, Dimension, Display, FlexDirection, JustifyContent, RectOffset};
use super::props::Style;

impl Style {
    /// Constructs a `Style` by parsing a whitespace-separated list of Tailwind-like class names.
    #[must_use]
    pub fn from_tailwind(classes: &str) -> Self {
        let mut computed_style = Self::default();
        for class in classes.split_whitespace() {
            match class {
                "flex" => computed_style.display = Display::Flex,
                "hidden" => computed_style.display = Display::None,

                "flex-row" | "row" => computed_style.flex_direction = FlexDirection::Row,
                "flex-col" | "col" => computed_style.flex_direction = FlexDirection::Column,

                "justify-start" => computed_style.justify_content = JustifyContent::Start,
                "justify-center" => computed_style.justify_content = JustifyContent::Center,
                "justify-end" => computed_style.justify_content = JustifyContent::End,
                "justify-between" => computed_style.justify_content = JustifyContent::SpaceBetween,

                "items-start" => computed_style.align_items = AlignItems::Start,
                "items-center" => computed_style.align_items = AlignItems::Center,
                "items-end" => computed_style.align_items = AlignItems::End,
                "items-stretch" => computed_style.align_items = AlignItems::Stretch,

                "w-full" => computed_style.width = Dimension::Percent(100.0),
                "h-full" => computed_style.height = Dimension::Percent(100.0),
                "w-auto" => computed_style.width = Dimension::Auto,
                "h-auto" => computed_style.height = Dimension::Auto,

                // Border radius
                "rounded-none" => computed_style.border_radius = 0.0,
                "rounded-sm" => computed_style.border_radius = 2.0,
                "rounded" => computed_style.border_radius = 4.0,
                "rounded-md" => computed_style.border_radius = 8.0,
                "rounded-lg" => computed_style.border_radius = 16.0,
                "rounded-xl" => computed_style.border_radius = 20.0,
                "rounded-2xl" => computed_style.border_radius = 24.0,
                "rounded-full" => computed_style.border_radius = 9999.0,

                other => {
                    // Custom parser for values like p-4, gap-2, w-[200], h-[100], etc.
                    if let Some(val) = other.strip_prefix("p-") {
                        if let Ok(pixels) = val.parse::<f32>() {
                            computed_style.padding = RectOffset::all(pixels * 4.0);
                        }
                    } else if let Some(val) = other.strip_prefix("px-") {
                        if let Ok(pixels) = val.parse::<f32>() {
                            computed_style.padding.left = pixels * 4.0;
                            computed_style.padding.right = pixels * 4.0;
                        }
                    } else if let Some(val) = other.strip_prefix("py-") {
                        if let Ok(pixels) = val.parse::<f32>() {
                            computed_style.padding.top = pixels * 4.0;
                            computed_style.padding.bottom = pixels * 4.0;
                        }
                    } else if let Some(val) = other.strip_prefix("m-") {
                        if let Ok(pixels) = val.parse::<f32>() {
                            computed_style.margin = RectOffset::all(pixels * 4.0);
                        }
                    } else if let Some(val) = other.strip_prefix("gap-") {
                        if let Ok(pixels) = val.parse::<f32>() {
                            computed_style.gap = pixels * 4.0;
                        }
                    } else if let Some(val) = other.strip_prefix("w-[") {
                        if let Some(end) = val.strip_suffix(']')
                            && let Ok(pixels) = end.parse::<f32>()
                        {
                            computed_style.width = Dimension::Pixels(pixels);
                        }
                    } else if let Some(val) = other.strip_prefix("h-[") {
                        if let Some(end) = val.strip_suffix(']')
                            && let Ok(pixels) = end.parse::<f32>()
                        {
                            computed_style.height = Dimension::Pixels(pixels);
                        }
                    } else if let Some(val) = other.strip_prefix("text-[")
                        && let Some(end) = val.strip_suffix(']')
                        && let Ok(sz) = end.parse::<f32>()
                    {
                        computed_style.text_size = sz;
                    }
                }
            }
        }
        computed_style
    }
}

/// Shorthand macro for `Style::from_tailwind(...)`.
#[macro_export]
macro_rules! tw {
    ($classes:expr_2021) => {
        $crate::style::Style::from_tailwind($classes)
    };
}
