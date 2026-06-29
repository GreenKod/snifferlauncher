use crate::core::ui::data_map::DataMap;
use crate::core::ui::event::EventBus;
use crate::core::ui::style_map::StyleMap;
use crate::core::ui::widget::WidgetId;
use crate::plugin::r#trait::UiPlugin;
use std::collections::HashMap;
use std::sync::Arc;

/// ID → Listener listesi eşlemesi.
///
/// Eklenti kayıt (`register`) karmaşıklığı: `O(subscriptions.len())` (amortised)
/// Olay dağıtım (`dispatch`) karmaşıklığı: `O(1)` `HashMap` lookup + `O(k)` dispatch
///   burada `k` = o ID'ye kayıtlı eklenti sayısı (genellikle 1-3)
///
/// # Kullanım
/// ```
/// let mut registry = PluginRegistry::default();
/// registry.register(Arc::new(HoverEffectPlugin));
/// // Her render frame'inde:
/// registry.dispatch(&bus, &styles, &data);
/// ```
#[derive(Default)]
pub struct PluginRegistry {
    /// Anahtar: `WidgetId` (u64) | Değer: o ID'yi dinleyen eklentiler
    subscriptions: HashMap<WidgetId, Vec<Arc<dyn UiPlugin>>>,
}

impl PluginRegistry {
    /// Eklentiyi yükle — her abone olduğu ID'ye "sticker yapıştır".
    pub fn register(&mut self, plugin: &Arc<dyn UiPlugin>) {
        for &id in plugin.subscriptions() {
            self.subscriptions
                .entry(id)
                .or_default()
                .push(Arc::clone(plugin));
        }
    }

    /// `EventBus`'tan gelen olayları ilgili eklentilere dağıt.
    ///
    /// Render döngüsünde her frame çağrılır.
    /// Kayıtlı olmayan ID'ler için O(1) miss — tam liste taranmaz.
    pub fn dispatch(&self, bus: &EventBus, styles: &StyleMap, data: &DataMap) {
        for event in bus.drain() {
            let id = event.widget_id();
            if let Some(listeners) = self.subscriptions.get(&id) {
                for plugin in listeners {
                    plugin.on_event(&event, styles, data);
                }
            }
        }
    }
}
