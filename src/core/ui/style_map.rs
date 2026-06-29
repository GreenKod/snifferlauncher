use crate::core::style::Style;
use crate::core::ui::widget::WidgetId;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Runtime stil override haritası.
///
/// `app.rs` widget ağacını statik olarak inşa eder; plugin'ler veya hover
/// mantığı `StyleMap` üzerinden belirli widget'ların stilini ezar.
/// Render aşamasında `draw_ui` önce `StyleMap`'i kontrol eder,
/// yoksa widget'ın kendi stilini kullanır.
///
/// `Arc<RwLock<_>>` ile birden fazla iş parçacığından güvenli erişim sağlanır:
/// - Okuma: render iş parçacığı (sık, bloklama olmadan)
/// - Yazma: plugin'ler / event bus dispatch (seyrek)
#[derive(Clone, Default)]
pub struct StyleMap {
    inner: Arc<RwLock<HashMap<WidgetId, Style>>>,
}

impl StyleMap {
    /// Belirli bir widget'ın stilini override et.
    pub fn set(&self, id: WidgetId, style: Style) {
        if let Ok(mut map) = self.inner.write() {
            map.insert(id, style);
        }
    }

    /// Widget'ın aktif override stilini döner (`None` → widget kendi stilini kullanır).
    #[must_use]
    pub fn get(&self, id: WidgetId) -> Option<Style> {
        self.inner.read().ok()?.get(&id).cloned()
    }

    /// Override'ı kaldır — widget kendi statik stiline geri döner.
    pub fn clear(&self, id: WidgetId) {
        if let Ok(mut map) = self.inner.write() {
            map.remove(&id);
        }
    }
}
