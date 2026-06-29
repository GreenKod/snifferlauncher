use crate::core::style::Style;
use crate::core::ui::data_map::{DataMap, DataValue};
use crate::core::ui::event::UiEvent;
use crate::core::ui::style_map::StyleMap;
use crate::core::ui::widget::{WidgetId, ids};
use crate::plugin::r#trait::UiPlugin;

/// Hover geri bildirim eklentisi.
///
/// `app.rs`'e tek satır kod yazmadan:
/// - Hover → vurgu rengi + "Açılıyor..." etiketi
/// - `HoverEnd` → orijinal stil ve etiket geri yüklenir
/// - Click → butonu geçici olarak gizler (UX geri bildirimi)
pub struct HoverEffectPlugin;

// Hover rengi sabitleri
const HOVER_BG_SETTINGS: u32 = 0x1A_6B_6B_FF; // koyu teal vurgu
const HOVER_BG_CONTACTS: u32 = 0x1A_6B_6B_FF;
const HOVER_BG_CAMERA: u32 = 0x1A_6B_6B_FF;

impl UiPlugin for HoverEffectPlugin {
    fn subscriptions(&self) -> &[WidgetId] {
        &[ids::BTN_SETTINGS, ids::BTN_CONTACTS, ids::BTN_CAMERA]
    }

    fn on_event(&self, event: &UiEvent, styles: &StyleMap, data: &DataMap) {
        match event {
            UiEvent::Hover(id) => {
                let hover_color = match *id {
                    ids::BTN_SETTINGS => HOVER_BG_SETTINGS,
                    ids::BTN_CONTACTS => HOVER_BG_CONTACTS,
                    ids::BTN_CAMERA => HOVER_BG_CAMERA,
                    _ => return,
                };
                styles.set(*id, Style::builder().background_color(hover_color).build());
                data.set(*id, "label", DataValue::Text("Açılıyor...".into()));
            }
            UiEvent::HoverEnd(id) => {
                styles.clear(*id);
                data.clear(*id, "label");
            }
            UiEvent::Click(id) => {
                // Tıklama geri bildirimi — buton 300ms sonra yeniden görünür
                // (gerçek zamanlı gecikmeler platform katmanında yönetilmeli)
                data.set(*id, "visible", DataValue::Visible(false));
            }
        }
    }
}
