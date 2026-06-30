use crate::core::ui::widget::WidgetId;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// A dynamic value that can be attached to any widget at runtime.
///
/// Plugins use this enum to override widget content and visibility
/// without modifying `app.rs`.
#[derive(Clone, Debug)]
pub enum DataValue {
    /// Override the label or button text (e.g. `"Start"` → `"Loading..."`).
    Text(String),
    /// Show or hide the widget.
    Visible(bool),
    /// Integer counter — badge counts, progress values, etc.
    Counter(i64),
    /// Arbitrary JSON string for future plugin-specific payloads.
    Custom(String),
}

/// Runtime data map — sibling of `StyleMap`, for **content** and **behaviour** overrides.
///
/// # Example
/// ```ignore
/// data.set(ids::BTN_SETTINGS, "label",   DataValue::Text("Loading...".into()));
/// data.set(ids::BTN_SETTINGS, "visible", DataValue::Visible(false));
/// data.clear(ids::BTN_SETTINGS, "label"); // revert to static label
/// ```
#[derive(Clone, Default)]
pub struct DataMap {
    inner: Arc<RwLock<HashMap<WidgetId, HashMap<&'static str, DataValue>>>>,
}

impl DataMap {
    /// Set a keyed value for a widget.
    ///
    /// Example: `data.set(BTN_SETTINGS, "label", DataValue::Text("Loading...".into()))`
    pub fn set(&self, id: WidgetId, key: &'static str, value: DataValue) {
        if let Ok(mut outer) = self.inner.write() {
            outer.entry(id).or_default().insert(key, value);
        }
    }

    /// Get a keyed value for a widget.
    #[must_use]
    pub fn get(&self, id: WidgetId, key: &'static str) -> Option<DataValue> {
        self.inner.read().ok()?.get(&id)?.get(key).cloned()
    }

    /// Remove a single key — the widget reverts to its static value.
    pub fn clear(&self, id: WidgetId, key: &'static str) {
        if let Ok(mut outer) = self.inner.write()
            && let Some(inner) = outer.get_mut(&id)
        {
            inner.remove(key);
        }
    }

    /// Remove all data entries for a widget.
    pub fn clear_widget(&self, id: WidgetId) {
        if let Ok(mut outer) = self.inner.write() {
            outer.remove(&id);
        }
    }

    /// Convenience: return the `Visible` value for `"visible"` key.
    /// Defaults to `true` if absent.
    #[must_use]
    pub fn is_visible(&self, id: WidgetId) -> bool {
        match self.get(id, "visible") {
            Some(DataValue::Visible(v)) => v,
            _ => true,
        }
    }

    /// Convenience: return the overridden label text, if any.
    #[must_use]
    pub fn label(&self, id: WidgetId) -> Option<String> {
        match self.get(id, "label") {
            Some(DataValue::Text(t)) => Some(t),
            _ => None,
        }
    }
}
