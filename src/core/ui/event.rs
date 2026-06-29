use crate::core::ui::widget::WidgetId;
use std::sync::{Arc, Mutex};

/// UI katmanından yayılan olaylar.
///
/// Platform dokunma/fare kodları bu enum'a dönüştürülür ve `EventBus`'a iletilir.
/// `PluginRegistry` her olayı ilgili eklentilere `O(1)` `HashMap` lookup ile dağıtır.
#[derive(Clone, Debug)]
pub enum UiEvent {
    /// Kullanıcı bir widget'a tıkladı / dokundu.
    Click(WidgetId),
    /// İşaretçi bir widget'ın üzerine geldi.
    Hover(WidgetId),
    /// İşaretçi bir widget'ın üzerinden ayrıldı.
    HoverEnd(WidgetId),
}

impl UiEvent {
    /// Olayın ait olduğu `WidgetId`'yi döner.
    #[must_use]
    pub const fn widget_id(&self) -> WidgetId {
        match self {
            Self::Click(id) | Self::Hover(id) | Self::HoverEnd(id) => *id,
        }
    }
}

/// Tüm platform kodları tarafından paylaşılan thread-safe olay kuyruğu.
///
/// Platform iş parçacığı (Android JNI, SDL event loop vb.) olayları
/// `push()` ile ekler; render döngüsü `drain()` ile tüketir.
#[derive(Clone, Default)]
pub struct EventBus {
    queue: Arc<Mutex<Vec<UiEvent>>>,
}

impl EventBus {
    /// Olayı kuyruğa ekle — platform iş parçacığından güvenli.
    pub fn push(&self, event: UiEvent) {
        if let Ok(mut q) = self.queue.lock() {
            q.push(event);
        }
    }

    /// Kuyruktaki tüm olayları tüket ve boşalt.
    /// Render döngüsünde her frame çağrılır.
    #[must_use]
    pub fn drain(&self) -> Vec<UiEvent> {
        self.queue
            .lock()
            .map_or_else(|_| Vec::new(), |mut q| std::mem::take(&mut *q))
    }
}
