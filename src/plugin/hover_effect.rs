use crate::core::ui::data_map::{DataMap, DataValue};
use crate::core::ui::event::UiEvent;
use crate::core::ui::style_map::StyleMap;
use crate::core::ui::widget::{WidgetId, ids};
use crate::plugin::r#trait::UiPlugin;

/// Hover feedback plugin.
///
/// Without modifying `app.rs`:
/// - `Hover`    → applies accent background color + sets "Opening..." label
/// - `HoverEnd` → restores original style and label
/// - `Click`    → temporarily hides the button as a UX tap feedback
pub struct HoverEffectPlugin;

// Hover color constants
const HOVER_BG: u32 = 0x1A_6B_6B_FF; // dark teal accent

impl UiPlugin for HoverEffectPlugin {
    fn subscriptions(&self) -> &[WidgetId] {
        &[ids::BTN_SETTINGS, ids::BTN_CONTACTS, ids::BTN_CAMERA]
    }

    fn on_event(
        &self,
        event: &UiEvent,
        styles: &StyleMap,
        data: &DataMap,
        _actions: &std::sync::Arc<std::sync::Mutex<Vec<crate::core::types::Action>>>,
    ) {
        match event {
            UiEvent::Hover(id, _w, _h, _mx, _my) => {
                styles.mutate(*id, |s| s.background_color = Some(HOVER_BG));
                data.set(*id, "label", DataValue::Text("Opening...".into()));
            }
            UiEvent::HoverEnd(id) => {
                styles.clear(*id);
                data.clear(*id, "label");
            }
            UiEvent::Click(id, _w, _h) => {
                // Tap feedback — hide the button; platform layer should restore after ~300 ms
                data.set(*id, "visible", DataValue::Visible(false));
            }
        }
    }
}
