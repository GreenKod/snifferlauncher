use crate::core::ui::widget::WidgetId;
use std::sync::{Arc, Mutex};

/// Events emitted from the UI layer.
///
/// Raw platform touch/mouse inputs are converted to this enum
/// and pushed to the `EventBus`. `PluginRegistry` dispatches each event
/// to the relevant plugins via `O(1)` `HashMap` lookup.
#[derive(Clone, Debug)]
pub enum UiEvent {
    /// User tapped or clicked a widget.
    Click(WidgetId),
    /// Pointer entered a widget.
    Hover(WidgetId),
    /// Pointer left a widget.
    HoverEnd(WidgetId),
}

impl UiEvent {
    /// Return the `WidgetId` this event belongs to.
    #[must_use]
    pub const fn widget_id(&self) -> WidgetId {
        match self {
            Self::Click(id) | Self::Hover(id) | Self::HoverEnd(id) => *id,
        }
    }
}

/// Thread-safe event queue shared by all platform backends.
///
/// Platform threads (Android JNI, SDL event loop, etc.) push events via
/// `push()`; the render loop consumes them each frame with `drain()`.
#[derive(Clone, Default)]
pub struct EventBus {
    queue: Arc<Mutex<Vec<UiEvent>>>,
}

impl EventBus {
    /// Enqueue an event — safe to call from any platform thread.
    pub fn push(&self, event: UiEvent) {
        if let Ok(mut q) = self.queue.lock() {
            q.push(event);
        }
    }

    /// Drain and return all queued events. Called once per render frame.
    #[must_use]
    pub fn drain(&self) -> Vec<UiEvent> {
        self.queue
            .lock()
            .map_or_else(|_| Vec::new(), |mut q| std::mem::take(&mut *q))
    }
}
