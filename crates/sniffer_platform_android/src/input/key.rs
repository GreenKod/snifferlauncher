use super::gestures;
use android_activity::InputStatus;
use android_activity::input::{KeyAction, KeyEvent, Keycode};
use sniffer_core::ui::event::UiEvent;

pub fn handle_key_event(key_event: &KeyEvent, state: &mut crate::app::AppState) -> InputStatus {
    if key_event.action() == KeyAction::Down {
        let keycode = key_event.key_code();
        if keycode == Keycode::Del {
            state.event_bus.push(UiEvent::Backspace);
        } else if let Some(c) = gestures::keycode_to_char(keycode) {
            state.event_bus.push(UiEvent::TextInput(c.to_string()));
        }
    }
    InputStatus::Handled
}
