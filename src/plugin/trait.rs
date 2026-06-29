use crate::core::ui::data_map::DataMap;
use crate::core::ui::event::UiEvent;
use crate::core::ui::style_map::StyleMap;
use crate::core::ui::widget::WidgetId;

/// Harici eklentilerin uygulaması gereken arayüz.
///
/// Bir plugin yüklenirken sistem `subscriptions()` metodunu okur ve
/// `PluginRegistry`'nin iç `HashMap`'ine `ID → [this_plugin]` kayıtlarını ekler.
/// Olay geldiğinde sadece ilgili ID'nin listener'ları çağrılır.
///
/// # Thread Safety
/// `Send + Sync` zorunludur; plugin'ler render iş parçacığından çağrılır
/// ancak başka iş parçacıklarıyla paylaşılabilir olmalıdır.
pub trait UiPlugin: Send + Sync {
    /// Bu plugin'in dinlemek istediği widget ID'leri.
    ///
    /// Slice döndürülür — heap allocation yok, sıfır maliyet.
    fn subscriptions(&self) -> &[WidgetId];

    /// İlgili bir `UiEvent` geldiğinde çağrılır.
    ///
    /// Plugin bu metot içinde `StyleMap` veya `DataMap` üzerinden
    /// widget'ların görünümünü ve içeriğini değiştirebilir.
    fn on_event(&self, event: &UiEvent, styles: &StyleMap, data: &DataMap);
}
