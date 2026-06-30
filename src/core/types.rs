use crate::core::style::Style;

pub type ElementChildren = Vec<Element>;

/// Identifies which button in the launcher was interacted with.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ButtonId {
    Settings,
    Contacts,
    Camera,
}

/// High-level action dispatched when the user clicks a button.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Action {
    OpenSettings,
    OpenContacts,
    OpenCamera,
}

/// Platform-agnostic UI tree node.
#[derive(Clone, Debug)]
pub enum Element {
    Container {
        style: Style,
        children: ElementChildren,
    },
    Button {
        id: ButtonId,
        title: String,
        style: Style,
        hovered: bool,
    },
    Label {
        text: String,
        style: Style,
    },
    Icon {
        id: ButtonId,
        style: Style,
    },
}

impl Element {
    /// Returns a reference to the style of any element variant.
    #[must_use]
    pub const fn style(&self) -> &Style {
        match self {
            Self::Container { style, .. }
            | Self::Button { style, .. }
            | Self::Label { style, .. }
            | Self::Icon { style, .. } => style,
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
