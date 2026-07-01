//! UI Event System and Dispatching
//!
//! This module defines the core event types used throughout the application to handle
//! user interaction (clicks, hovers). Events are completely decoupled from the rendering
//! and platform-specific input pipelines.
//!
//! The `EventBus` acts as a central queue where platform layers (like Android `MotionEvents`
//! or Desktop Mouse events) push interactions, and the `PluginRegistry` drains them
//! before each render frame to invoke plugin behaviors.

use crate::core::ui::widget::WidgetId;
use std::sync::{Arc, Mutex};

/// Represents a distinct user interaction with a UI widget.
///
/// Raw platform touch/mouse inputs are converted to this enum
/// and pushed to the `EventBus`. `PluginRegistry` dispatches each event
/// to the relevant plugins via `O(1)` `HashMap` lookup.
#[derive(Clone, Debug)]
pub enum UiEvent {
    /// User tapped or clicked a widget. Contains ID, width, height.
    Click(WidgetId, f32, f32),
    /// Pointer entered a widget. Contains ID, width, height.
    Hover(WidgetId, f32, f32, f32, f32),
    /// Pointer left a widget.
    HoverEnd(WidgetId),
}

impl UiEvent {
    /// Return the `WidgetId` this event belongs to.
    #[must_use]
    pub const fn widget_id(&self) -> WidgetId {
        match self {
            Self::Click(id, _, _) | Self::Hover(id, _, _, _, _) | Self::HoverEnd(id) => *id,
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
