use crate::core::ui::widget::WidgetId;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Bir widget'a bağlanabilen dinamik veri değeri.
///
/// Plugin'ler bu enum aracılığıyla herhangi bir widget'ın içeriğini
/// ve görünürlüğünü `app.rs`'e dokunmadan değiştirebilir.
#[derive(Clone, Debug)]
pub enum DataValue {
    /// Label veya buton metnini override et. (`"Başla"` → `"Yükleniyor..."`)
    Text(String),
    /// Widget'ı görünür veya gizli yap.
    Visible(bool),
    /// Rozet sayacı, ilerleme yüzdesi vb. tamsayı değer.
    Counter(i64),
    /// Gelecekteki esneklik için JSON string — özel eklenti verileri.
    Custom(String),
}

/// Widget ID'sine ve anahtar adına göre dinamik değerleri depolar.
///
/// `StyleMap`'in kardeşi — stil değil, **içerik** ve **davranış** override'ları için.
///
/// # Örnek
/// ```
/// data.set(ids::BTN_SETTINGS, "label",   DataValue::Text("Yükleniyor...".into()));
/// data.set(ids::BTN_SETTINGS, "visible", DataValue::Visible(false));
/// data.clear(ids::BTN_SETTINGS, "label"); // orijinal metne dön
/// ```
#[derive(Clone, Default)]
pub struct DataMap {
    inner: Arc<RwLock<HashMap<WidgetId, HashMap<&'static str, DataValue>>>>,
}

impl DataMap {
    /// Belirli bir widget için `key` anahtarlı değeri ayarla.
    pub fn set(&self, id: WidgetId, key: &'static str, value: DataValue) {
        if let Ok(mut outer) = self.inner.write() {
            outer.entry(id).or_default().insert(key, value);
        }
    }

    /// Belirli bir widget için `key` anahtarlı değeri al.
    #[must_use]
    pub fn get(&self, id: WidgetId, key: &'static str) -> Option<DataValue> {
        self.inner.read().ok()?.get(&id)?.get(key).cloned()
    }

    /// Belirli anahtarı sil — widget statik değerine geri döner.
    pub fn clear(&self, id: WidgetId, key: &'static str) {
        if let Ok(mut outer) = self.inner.write()
            && let Some(inner) = outer.get_mut(&id)
        {
            inner.remove(key);
        }
    }

    /// Widget'a ait tüm değerleri temizle.
    pub fn clear_widget(&self, id: WidgetId) {
        if let Ok(mut outer) = self.inner.write() {
            outer.remove(&id);
        }
    }

    /// `"visible"` anahtarındaki `DataValue::Visible` değerini döner.
    /// Yoksa `true` (varsayılan görünür) döner.
    #[must_use]
    pub fn is_visible(&self, id: WidgetId) -> bool {
        match self.get(id, "visible") {
            Some(DataValue::Visible(v)) => v,
            _ => true,
        }
    }

    /// `"label"` anahtarındaki `DataValue::Text` değerini döner.
    /// Yoksa `None` döner (widget kendi statik labelını kullanır).
    #[must_use]
    pub fn label(&self, id: WidgetId) -> Option<String> {
        match self.get(id, "label") {
            Some(DataValue::Text(t)) => Some(t),
            _ => None,
        }
    }
}
