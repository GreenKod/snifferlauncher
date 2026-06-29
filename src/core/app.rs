use crate::core::Point;
use crate::core::ScreenMetrics;
use crate::core::style::Style;
use crate::core::types::{Action, Application, ButtonId, Element};

#[derive(Default, Debug)]
pub struct LauncherState {
    pub hovered: Option<ButtonId>,
}

#[derive(Clone, Debug)]
pub enum LauncherMessage {
    PointerMoved(Point),
    ButtonHovered(Option<ButtonId>),
    ButtonClicked(ButtonId),
}

pub struct LauncherApp;

/// Responsive layout tokens derived from `ScreenMetrics`.
///
/// All values are in **physical pixels** (already scaled by the DPI factor).
/// - Compact  : `logical_width` < 360 dp  (small/old phones, ~320–359 dp)
/// - Normal   : 360–599 dp              (standard Android phones)
/// - Large    : ≥ 600 dp                (tablets, foldables)
struct Tokens {
    /// Outer container padding (all sides)
    panel_padding: f32,
    /// Gap between the header card and the button card
    section_gap: f32,
    /// Inner button-list padding (all sides)
    card_padding: f32,
    /// Gap between buttons inside the card
    button_gap: f32,
    /// Fixed height of each button row
    button_height: f32,
    /// Inner horizontal/vertical padding on each button
    button_px: f32,
    button_py: f32,
    /// Header card inner padding
    header_padding: f32,
    /// Main title font size (sp)
    title_size: f32,
    /// Subtitle font size (sp)
    subtitle_size: f32,
    /// Border radii
    card_radius: f32,
    button_radius: f32,
}

impl Tokens {
    fn from_metrics(m: &ScreenMetrics) -> Self {
        if m.is_very_compact() {
            // ── Very Compact (< 320 logical dp) ────────────────────────────
            Self {
                panel_padding: m.dp(8.0),
                section_gap: m.dp(6.0),
                card_padding: m.dp(6.0),
                button_gap: m.dp(6.0),
                button_height: m.dp(60.0),
                button_px: m.dp(8.0),
                button_py: m.dp(10.0),
                header_padding: m.dp(10.0),
                title_size: m.sp(14.0),
                subtitle_size: m.sp(9.0),
                card_radius: m.dp(6.0),
                button_radius: m.dp(4.0),
            }
        } else if m.is_compact() {
            // ── Compact (320–359 logical dp) ────────────────────────────────
            Self {
                panel_padding: m.dp(12.0),
                section_gap: m.dp(10.0),
                card_padding: m.dp(8.0),
                button_gap: m.dp(8.0),
                button_height: m.dp(72.0),
                button_px: m.dp(10.0),
                button_py: m.dp(14.0),
                header_padding: m.dp(14.0),
                title_size: m.sp(16.0),
                subtitle_size: m.sp(10.0),
                card_radius: m.dp(12.0),
                button_radius: m.dp(8.0),
            }
        } else if m.is_large() {
            // ── Large (≥ 600 logical dp) ────────────────────────────────────
            Self {
                panel_padding: m.dp(32.0),
                section_gap: m.dp(24.0),
                card_padding: m.dp(20.0),
                button_gap: m.dp(20.0),
                button_height: m.dp(104.0),
                button_px: m.dp(20.0),
                button_py: m.dp(28.0),
                header_padding: m.dp(28.0),
                title_size: m.sp(26.0),
                subtitle_size: m.sp(15.0),
                card_radius: m.dp(24.0),
                button_radius: m.dp(18.0),
            }
        } else {
            // ── Normal (360–599 logical dp) — baseline ──────────────────────
            Self {
                panel_padding: m.dp(22.0),
                section_gap: m.dp(16.0),
                card_padding: m.dp(10.0),
                button_gap: m.dp(12.0),
                button_height: m.dp(86.0),
                button_px: m.dp(14.0),
                button_py: m.dp(20.0),
                header_padding: m.dp(22.0),
                title_size: m.sp(20.0),
                subtitle_size: m.sp(12.0),
                card_radius: m.dp(24.0),
                button_radius: m.dp(18.0),
            }
        }
    }
}

impl Application for LauncherApp {
    type Message = LauncherMessage;
    type State = LauncherState;

    fn update(state: &mut Self::State, msg: Self::Message) -> Option<Action> {
        match msg {
            LauncherMessage::PointerMoved(_point) => None,
            LauncherMessage::ButtonHovered(btn_id) => {
                state.hovered = btn_id;
                None
            }
            LauncherMessage::ButtonClicked(btn_id) => match btn_id {
                ButtonId::Settings => Some(Action::OpenSettings),
                ButtonId::Contacts => Some(Action::OpenContacts),
                ButtonId::Camera => Some(Action::OpenCamera),
            },
        }
    }

    fn view(state: &Self::State, metrics: &ScreenMetrics) -> Element {
        let t = Tokens::from_metrics(metrics);

        let outer_style = Style::builder()
            .background_color(crate::core::style::BACKGROUND)
            .width_percent(100.0)
            .height_percent(100.0)
            .padding_all(t.panel_padding)
            .gap(t.section_gap)
            .column()
            .align_stretch()
            .build();

        Element::Container {
            style: outer_style,
            children: vec![build_header(&t), build_button_list(state, &t)],
        }
    }
}

fn build_header(t: &Tokens) -> Element {
    let header_style = Style::builder()
        .background_color(crate::core::style::HEADER_SURFACE)
        .border_radius(t.card_radius)
        .padding_all(t.header_padding)
        .column()
        .align_start()
        .gap(t.dp(8.0))
        .build();

    Element::Container {
        style: header_style,
        children: vec![
            Element::Label {
                text: "SNIFFER LAUNCHER".to_string(),
                style: Style::builder()
                    .text_color(crate::core::style::BUTTON_TEXT)
                    .text_size(t.title_size)
                    .build(),
            },
            Element::Label {
                text: "Platform-Agnostic UI System (v1.0)".to_string(),
                style: Style::builder()
                    .text_color(crate::core::style::BUTTON_MUTED)
                    .text_size(t.subtitle_size)
                    .build(),
            },
        ],
    }
}

fn build_button_list(state: &LauncherState, t: &Tokens) -> Element {
    let card_style = Style::builder()
        .background_color(crate::core::style::HEADER_SURFACE)
        .border_radius(t.card_radius)
        .padding_all(t.card_padding)
        .column()
        .align_start()
        .gap(t.button_gap)
        .build();

    let make_button = |id: ButtonId, bg: &str, title: &str, hovered: bool| {
        let btn_style = Style::builder()
            .background_color_from_tailwind(bg)
            .border_radius(t.button_radius)
            .padding_horizontal(t.button_px)
            .padding_vertical(t.button_py)
            .shadow_card()
            .height_pixels(t.button_height)
            .build();
        Element::Button {
            id,
            title: title.to_string(),
            style: btn_style,
            hovered,
        }
    };

    let settings_bg = if state.hovered == Some(ButtonId::Settings) {
        "bg-settings-hover"
    } else {
        "bg-settings"
    };
    let contacts_bg = if state.hovered == Some(ButtonId::Contacts) {
        "bg-contacts-hover"
    } else {
        "bg-contacts"
    };
    let camera_bg = if state.hovered == Some(ButtonId::Camera) {
        "bg-camera-hover"
    } else {
        "bg-camera"
    };

    Element::Container {
        style: card_style,
        children: vec![
            make_button(
                ButtonId::Settings,
                settings_bg,
                "Settings",
                state.hovered == Some(ButtonId::Settings),
            ),
            make_button(
                ButtonId::Contacts,
                contacts_bg,
                "Contacts",
                state.hovered == Some(ButtonId::Contacts),
            ),
            make_button(
                ButtonId::Camera,
                camera_bg,
                "Camera",
                state.hovered == Some(ButtonId::Camera),
            ),
        ],
    }
}
// ── Helper: dp() from raw f32 ─────────────────────────────────────────────────
// `t.dp(x)` is already scaled, but for gaps used inline we forward the method.
impl Tokens {
    #[allow(clippy::unused_self)]
    const fn dp(&self, v: f32) -> f32 {
        // Intentionally a no-op: all Tokens fields are pre-scaled in from_metrics.
        // This method is kept so inline `t.dp(...)` calls in view() remain readable.
        v
    }
}
