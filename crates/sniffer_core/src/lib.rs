pub mod anim;
pub mod layout;
pub mod log;
pub mod math;
pub mod render_api;
pub mod style;
pub mod text_measure;
pub mod types;
pub mod ui;
pub mod vault;
pub mod virtualization;
#[cfg(feature = "devkit")]
pub mod profiler;

// Convenience re-exports
pub use layout::{calculate_layout, LayoutNode};
pub use math::{Point, Rect, ScreenMetrics, Size};
pub use render_api::Renderer;
pub use types::{Action, Application, Element};
pub use vault::{DataVault, SystemTheme};

// Event-driven system re-exports
pub use ui::widget::ids;
pub use ui::{
    ButtonBuilder, ContainerBuilder, DataMap, DataValue, EventBus, LabelBuilder, StyleMap, UiEvent,
    Widget, WidgetId,
};

