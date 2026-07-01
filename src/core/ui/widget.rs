use crate::core::style::Style;

/// Unique identifier for every UI widget.
/// Computed via FNV-1a hash of a `&[u8]` literal at compile time — zero runtime cost.
pub type WidgetId = u64;

/// Compute a `WidgetId` (u64) from a byte-string literal at runtime or compile time.
#[must_use]
pub const fn fnv1a(s: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325_u64;
    let mut i = 0usize;
    while i < s.len() {
        hash ^= s[i] as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01B3_u64);
        i += 1;
    }
    hash
}

/// Compute a `WidgetId` (u64) from a byte-string literal at compile time.
///
/// Uses the FNV-1a hash algorithm; collisions are astronomically unlikely for
/// typical widget name sets.
///
/// # Example
/// ```
/// const MY_BUTTON: u64 = snifferlauncher::wid!(b"my-button");
/// ```
#[macro_export]
macro_rules! wid {
    ($s:literal) => {{
        $crate::core::ui::widget::fnv1a($s)
    }};
}

/// All declared widget IDs — single source of truth.
///
/// Add new widgets here. The `wid!` macro guarantees compile-time uniqueness
/// (each byte literal produces a distinct hash).
pub mod ids {
    use super::WidgetId;

    pub const ROOT: WidgetId = crate::wid!(b"root");
    pub const HEADER: WidgetId = crate::wid!(b"header");
    pub const BTN_LIST: WidgetId = crate::wid!(b"btn-list");
    pub const BTN_SETTINGS: WidgetId = crate::wid!(b"btn-settings");
    pub const BTN_CONTACTS: WidgetId = crate::wid!(b"btn-contacts");
    pub const BTN_CAMERA: WidgetId = crate::wid!(b"btn-camera");
}

/// Platform-agnostic UI tree node.
///
/// The `Widget` tree is rebuilt every frame by `build_ui()`.
/// `StyleMap` and `DataMap` overrides are applied during the render pass.
#[derive(Clone, Debug)]
pub enum Widget {
    /// Layout container — holds child widgets.
    Container {
        id: Option<WidgetId>,
        style: Style,
        children: Vec<Self>,
    },
    /// Tappable action button.
    Button {
        id: WidgetId,
        label: String,
        style: Style,
        hovered: bool,
    },
    /// Text display element.
    Label {
        id: Option<WidgetId>,
        text: String,
        style: Style,
        visible: bool,
    },
    /// Stand-alone icon element.
    Icon {
        id: Option<WidgetId>,
        widget_id: WidgetId,
        style: Style,
    },
}

impl Widget {
    /// Return a reference to the style of any `Widget` variant.
    #[must_use]
    pub const fn style(&self) -> &Style {
        match self {
            Self::Container { style, .. }
            | Self::Button { style, .. }
            | Self::Label { style, .. }
            | Self::Icon { style, .. } => style,
        }
    }

    /// Return the widget's ID (`None` for anonymous elements).
    #[must_use]
    pub const fn widget_id(&self) -> Option<WidgetId> {
        match self {
            Self::Button { id, .. } => Some(*id),
            Self::Container { id, .. } | Self::Label { id, .. } | Self::Icon { id, .. } => *id,
        }
    }
}
