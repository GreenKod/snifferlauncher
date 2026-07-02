use crate::core::style::Style;

use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicU32;

pub static SCREEN_WIDTH: AtomicU32 = AtomicU32::new(0);
pub static SCREEN_HEIGHT: AtomicU32 = AtomicU32::new(0);

pub type ElementChildren = Vec<Element>;

/// High-level action dispatched by the host or plugins.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Action {
    OpenSettings,
    OpenContacts,
    OpenCamera,
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
}

impl Element {
    /// Returns a reference to the style of any element variant.
    #[must_use]
    pub const fn style(&self) -> &Style {
        match self {
            Self::Container { style, .. } | Self::Label { style, .. } => style,
        }
    }

    /// Returns the ID of the element, if any.
    #[must_use]
    pub fn id(&self) -> Option<&str> {
        match self {
            Self::Container { id, .. } | Self::Label { id, .. } => id.as_deref(),
        }
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
