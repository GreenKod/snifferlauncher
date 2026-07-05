pub mod anim;
pub mod layout;
pub mod render;
pub mod style;
pub mod text_measure;
pub mod types;
pub mod ui;

// Convenience re-exports — keeps all call sites working without path changes.
pub use layout::{LayoutNode, calculate_layout};
pub use render::math::geometry::{Point, Rect, ScreenMetrics, Size};
pub use render::{GlowRenderer, Renderer};
pub use types::{Action, Application, Element};

// New event-driven system re-exports
pub use ui::widget::ids;
pub use ui::{
    ButtonBuilder, ContainerBuilder, DataMap, DataValue, EventBus, LabelBuilder, StyleMap, UiEvent,
    Widget, WidgetId,
};
