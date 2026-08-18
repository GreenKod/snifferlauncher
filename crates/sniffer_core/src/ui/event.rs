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
    PointerDown(u64),
    PointerUp(Option<u64>),
    Scroll(Option<u64>, f32, f32, f32, f32),
    WindowResized(f32, f32),
    /// Rust scroll physics: a page snapping animation has completed.
    /// `widget_id`: FNV-1a hash ID of the ScrollView, `page`: 0-based page index.
    PageSnapped {
        widget_id: u64,
        page: i32,
    },
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
