use crate::core::style::Style;
use crate::core::ui::widget::WidgetId;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Runtime style override map.
///
/// `app.rs` builds the widget tree statically; plugins or hover logic use
/// `StyleMap` to override the style of specific widgets at runtime.
/// During the render pass, `draw_ui` checks `StyleMap` first — if an override
/// is present it takes precedence over the widget's own baked-in style.
///
/// `Arc<RwLock<_>>` provides thread-safe access:
/// - Readers: render thread (frequent, non-blocking)
/// - Writers: plugins / event-bus dispatch (infrequent)
#[derive(Clone, Default)]
pub struct StyleMap {
    inner: Arc<RwLock<HashMap<WidgetId, Style>>>,
}

impl StyleMap {
    /// Override the style for a specific widget.
    pub fn set(&self, id: WidgetId, style: Style) {
        if let Ok(mut map) = self.inner.write() {
            map.insert(id, style);
        }
    }

    /// Return the active override style for a widget (`None` means use its own style).
    #[must_use]
    pub fn get(&self, id: WidgetId) -> Option<Style> {
        self.inner.read().ok()?.get(&id).cloned()
    }

    /// Remove the override — the widget falls back to its static style.
    pub fn clear(&self, id: WidgetId) {
        if let Ok(mut map) = self.inner.write() {
            map.remove(&id);
        }
    }
}
