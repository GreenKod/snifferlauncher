use crate::core::style::Style;

use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicU32;

pub static SCREEN_WIDTH: AtomicU32 = AtomicU32::new(0);
pub static SCREEN_HEIGHT: AtomicU32 = AtomicU32::new(0);

pub type ElementChildren = Vec<Element>;

/// High-level action dispatched by the host or plugins.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Action {
    OpenSettings,
    OpenContacts,
    OpenCamera,
    LoadImage { id: String, src: String },
    FocusTextInput(String),
    BlurTextInput,
}

/// A sample state structure to demonstrate `Bincode` / `ArrayBuffer` passing.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppState {
    pub click_count: u32,
    pub screen_width: f32,
    pub screen_height: f32,
}

/// Platform-agnostic UI tree node.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Element {
    Container {
        id: Option<String>,
        style: Style,
        children: ElementChildren,
    },
    Label {
        id: Option<String>,
        text: String,
        style: Style,
    },
    Image {
        id: Option<String>,
        src: String,
        style: Style,
    },
    TextInput {
        id: Option<String>,
        value: String,
        focused: bool,
        style: Style,
    },
    ScrollView {
        id: Option<String>,
        style: Style,
        children: ElementChildren,
        scroll_x: f32,
        scroll_y: f32,
        scroll_sensitivity: Option<f32>,
        dynamic_sensitivity: Option<bool>,
        momentum_scrolling: Option<bool>,
        capture_drag: Option<bool>,
    },
    Checkbox {
        id: Option<String>,
        checked: bool,
        style: Style,
    },
    Slider {
        id: Option<String>,
        value: f32,
        min: f32,
        max: f32,
        style: Style,
    },
    ProgressBar {
        id: Option<String>,
        value: f32,
        max: f32,
        style: Style,
    },
}

impl Element {
    /// Returns a reference to the style of any element variant.
    #[must_use]
    pub const fn style(&self) -> &Style {
        match self {
            Self::Container { style, .. }
            | Self::Label { style, .. }
            | Self::Image { style, .. }
            | Self::TextInput { style, .. }
            | Self::ScrollView { style, .. }
            | Self::Checkbox { style, .. }
            | Self::Slider { style, .. }
            | Self::ProgressBar { style, .. } => style,
        }
    }

    /// Returns the ID of the element, if any.
    #[must_use]
    pub fn id(&self) -> Option<&str> {
        match self {
            Self::Container { id, .. }
            | Self::Label { id, .. }
            | Self::Image { id, .. }
            | Self::TextInput { id, .. }
            | Self::ScrollView { id, .. }
            | Self::Checkbox { id, .. }
            | Self::Slider { id, .. }
            | Self::ProgressBar { id, .. } => id.as_deref(),
        }
    }

    /// Mutates a style property of the element or its children with the matching ID.
    pub fn mutate_style(&mut self, target_id: &str, property: &str, value: &str) -> bool {
        if self.id() == Some(target_id) {
            let style = match self {
                Self::Container { style, .. }
                | Self::Label { style, .. }
                | Self::Image { style, .. }
                | Self::TextInput { style, .. }
                | Self::ScrollView { style, .. }
                | Self::Checkbox { style, .. }
                | Self::Slider { style, .. }
                | Self::ProgressBar { style, .. } => style,
            };

            // Simple property mapping
            match property {
                "background_color" => {
                    if let Ok(color) = u32::from_str_radix(value.trim_start_matches('#'), 16) {
                        style.background_color = Some(color);
                    }
                }
                "text_color" => {
                    if let Ok(color) = u32::from_str_radix(value.trim_start_matches('#'), 16) {
                        style.text_color = Some(color);
                    }
                }
                "text_size" => {
                    if let Ok(size) = value.parse::<f32>() {
                        style.text_size = size;
                    }
                }
                "width" => {
                    if let Ok(w) = value.parse::<f32>() {
                        style.width = crate::core::style::Dimension::Pixels(w);
                    } else if value == "auto" {
                        style.width = crate::core::style::Dimension::Auto;
                    } else if let Some(p) = value.strip_suffix('%')
                        && let Ok(pct) = p.parse::<f32>()
                    {
                        style.width = crate::core::style::Dimension::Percent(pct);
                    }
                }
                "height" => {
                    if let Ok(h) = value.parse::<f32>() {
                        style.height = crate::core::style::Dimension::Pixels(h);
                    } else if value == "auto" {
                        style.height = crate::core::style::Dimension::Auto;
                    } else if let Some(p) = value.strip_suffix('%')
                        && let Ok(pct) = p.parse::<f32>()
                    {
                        style.height = crate::core::style::Dimension::Percent(pct);
                    }
                }
                // Add more properties as needed
                _ => return false,
            }
            return true;
        }

        if let Self::Container { children, .. } | Self::ScrollView { children, .. } = self {
            for child in children {
                if child.mutate_style(target_id, property, value) {
                    return true;
                }
            }
        }
        false
    }

    /// Mutates the text of a Label element with the matching ID.
    pub fn mutate_text(&mut self, target_id: &str, new_text: &str) -> bool {
        if self.id() == Some(target_id) {
            match self {
                Self::Label { text, .. } => {
                    *text = new_text.to_string();
                    return true;
                }
                Self::TextInput { value, .. } => {
                    *value = new_text.to_string();
                    return true;
                }
                _ => {}
            }
        }

        if let Self::Container { children, .. } | Self::ScrollView { children, .. } = self {
            for child in children {
                if child.mutate_text(target_id, new_text) {
                    return true;
                }
            }
        }
        false
    }

    /// Mutates the state (e.g. `scroll_offset`, checked, value) of an element with the matching ID.
    pub fn mutate_state(&mut self, target_id: &str, property: &str, new_val: f32) -> bool {
        if self.id() == Some(target_id) {
            match self {
                Self::ScrollView {
                    scroll_x, scroll_y, ..
                } => {
                    if property == "scroll_x" {
                        *scroll_x = new_val;
                        return true;
                    } else if property == "scroll_y" {
                        *scroll_y = new_val;
                        return true;
                    }
                }
                Self::Checkbox { checked, .. } if property == "checked" => {
                    *checked = new_val > 0.0;
                    return true;
                }
                Self::Slider { value, .. } | Self::ProgressBar { value, .. }
                    if property == "value" =>
                {
                    *value = new_val;
                    return true;
                }
                _ => {}
            }
        }

        if let Self::Container { children, .. } | Self::ScrollView { children, .. } = self {
            for child in children {
                if child.mutate_state(target_id, property, new_val) {
                    return true;
                }
            }
        }
        false
    }

    /// Inserts a child element into a Container with the matching ID.
    pub fn insert_child(&mut self, parent_id: &str, child: Self) -> bool {
        if self.id() == Some(parent_id)
            && let Self::Container { children, .. } | Self::ScrollView { children, .. } = self
        {
            children.push(child);
            return true;
        }

        if let Self::Container { children, .. } | Self::ScrollView { children, .. } = self {
            for c in children {
                if c.insert_child(parent_id, child.clone()) {
                    return true;
                }
            }
        }
        false
    }

    /// Removes a child node with the matching ID from the tree.
    pub fn remove_node(&mut self, target_id: &str) -> bool {
        if let Self::Container { children, .. } | Self::ScrollView { children, .. } = self {
            if let Some(pos) = children.iter().position(|c| c.id() == Some(target_id)) {
                children.remove(pos);
                return true;
            }
            for child in children {
                if child.remove_node(target_id) {
                    return true;
                }
            }
        }
        false
    }
}

/// Core application trait — implemented by platform-agnostic app logic.
pub trait Application {
    type Message: Clone + std::fmt::Debug;
    type State: Default;

    fn update(state: &mut Self::State, msg: Self::Message) -> Option<Action>;

    /// Build the UI element tree for the current state and screen metrics.
    ///
    /// `metrics` carries the DPI scale factor and logical screen dimensions so
    /// the view can adapt padding, font sizes, and element heights to the device.
    fn view(state: &Self::State, metrics: &crate::core::ScreenMetrics) -> Element;
}
