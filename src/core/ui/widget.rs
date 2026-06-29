use crate::core::style::Style;

/// Her widget'ın benzersiz kimliği.
/// FNV-1a hash of &'static str — derleme zamanında hesaplanır, runtime maliyeti sıfır.
pub type WidgetId = u64;

/// Derleme zamanında bir string literal'i `WidgetId` (u64) değerine dönüştürür.
///
/// # Örnek
/// ```
/// const MY_BUTTON: u64 = snifferlauncher::wid!(b"my-button");
/// ```
#[macro_export]
macro_rules! wid {
    ($s:literal) => {{
        const fn fnv1a(s: &[u8]) -> u64 {
            let mut hash: u64 = 0xcbf2_9ce4_8422_2325_u64;
            let mut i = 0usize;
            while i < s.len() {
                hash ^= s[i] as u64;
                hash = hash.wrapping_mul(0x0000_0100_0000_01B3_u64);
                i += 1;
            }
            hash
        }
        fnv1a($s)
    }};
}

/// Tüm tanımlı widget ID'leri — merkezi tek kaynak.
///
/// Yeni bir widget eklendiğinde buraya eklenir; `wid!` makrosu çakışmaları
/// derleme zamanında önler (her literal benzersiz hash üretir).
pub mod ids {
    use super::WidgetId;

    pub const ROOT: WidgetId = crate::wid!(b"root");
    pub const HEADER: WidgetId = crate::wid!(b"header");
    pub const BTN_LIST: WidgetId = crate::wid!(b"btn-list");
    pub const BTN_SETTINGS: WidgetId = crate::wid!(b"btn-settings");
    pub const BTN_CONTACTS: WidgetId = crate::wid!(b"btn-contacts");
    pub const BTN_CAMERA: WidgetId = crate::wid!(b"btn-camera");
}

/// Platform-agnostik UI ağaç düğümü.
///
/// `Widget` ağacı her frame `build_ui()` tarafından yeniden inşa edilir;
/// `StyleMap` ve `DataMap` override'ları render aşamasında uygulanır.
#[derive(Clone, Debug)]
pub enum Widget {
    /// İç içe yerleşim düğümü.
    Container {
        id: Option<WidgetId>,
        style: Style,
        children: Vec<Self>,
    },
    /// Tıklanabilir aksiyon butonu.
    Button {
        id: WidgetId,
        label: String,
        style: Style,
        hovered: bool,
    },
    /// Sadece metin gösterimi.
    Label {
        id: Option<WidgetId>,
        text: String,
        style: Style,
        visible: bool,
    },
    /// Bağımsız ikon elemanı.
    Icon {
        id: Option<WidgetId>,
        widget_id: WidgetId, // hangi buton ikonunu çizeceğini belirtir
        style: Style,
    },
}

impl Widget {
    /// Herhangi bir `Widget` varyantının stiline referans döner.
    #[must_use]
    pub const fn style(&self) -> &Style {
        match self {
            Self::Container { style, .. }
            | Self::Button { style, .. }
            | Self::Label { style, .. }
            | Self::Icon { style, .. } => style,
        }
    }

    /// Widget'ın ID'sini döner (`None` ise anonim elemandır).
    #[must_use]
    pub const fn widget_id(&self) -> Option<WidgetId> {
        match self {
            Self::Button { id, .. } => Some(*id),
            Self::Container { id, .. } | Self::Label { id, .. } | Self::Icon { id, .. } => *id,
        }
    }
}
