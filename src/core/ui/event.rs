use serde::{Deserialize, Serialize};
use std::cell::RefCell;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum UiEvent {
    Click(u64, f32, f32),
    ClickOutside,
    Hover(u64, f32, f32, f32, f32),
    HoverEnd(u64),
    TextInput(String),
    Backspace,
    Scroll(Option<u64>, f32, f32),
}

#[derive(Default)]
pub struct EventBus {
    events: RefCell<Vec<UiEvent>>,
}

impl EventBus {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&self, event: UiEvent) {
        self.events.borrow_mut().push(event);
    }

    #[must_use]
    pub fn drain(&self) -> Vec<UiEvent> {
        let mut borrowed = self.events.borrow_mut();
        std::mem::take(&mut borrowed)
    }
}
