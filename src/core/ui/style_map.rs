use crate::core::style::StyleOverride;
use crate::core::ui::widget::WidgetId;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone, Default)]
pub struct StyleMap {
    inner: Arc<RwLock<HashMap<WidgetId, StyleOverride>>>,
}

impl StyleMap {
    pub fn set(&self, id: WidgetId, override_style: StyleOverride) {
        if let Ok(mut map) = self.inner.write() {
            map.insert(id, override_style);
        }
    }

    #[must_use]
    pub fn get(&self, id: WidgetId) -> Option<StyleOverride> {
        self.inner.read().ok()?.get(&id).cloned()
    }

    /// Mutate an existing override or insert a new default one, then apply closure
    pub fn mutate<F>(&self, id: WidgetId, f: F)
    where
        F: FnOnce(&mut StyleOverride),
    {
        if let Ok(mut map) = self.inner.write() {
            let entry = map.entry(id).or_default();
            f(entry);
        }
    }

    pub fn clear(&self, id: WidgetId) {
        if let Ok(mut map) = self.inner.write() {
            map.remove(&id);
        }
    }
}
