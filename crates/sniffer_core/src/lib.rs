pub mod anim;
pub mod layout;
pub mod log;
pub mod math;
pub mod physics;
#[cfg(feature = "devkit")]
pub mod profiler;
pub mod render;
pub mod style;
pub mod text;
pub mod types;
pub mod ui;
pub mod vault;
pub mod virtualization;

// Convenience re-exports
pub use layout::{LayoutNode, calculate_layout};
pub use math::{Point, Rect, ScreenMetrics, Size};
pub use render::Renderer;
pub use types::{Action, Element};
pub use vault::{DataVault, SystemTheme};

// Event-driven system re-exports
pub use ui::{DataMap, DataValue, EventBus, StyleMap, UiEvent, WidgetId};
