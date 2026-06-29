use crate::core::types::{Action, Application, ButtonId, Element};
use crate::core::style::Style;
use crate::core::Point;

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

    fn view(state: &Self::State) -> Element {
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
            style: Style::from_tailwind(
                "bg-background w-full h-full p-22 gap-16 col items-stretch",
            ),
            children: vec![
                // Hero Header Card
                Element::Container {
                    style: Style::from_tailwind("bg-header rounded-2xl p-22 col items-start gap-8"),
                    children: vec![
                        Element::Label {
                            text: "SNIFFER LAUNCHER".to_string(),
                            style: Style::from_tailwind("text-main text-[20]"),
                        },
                        Element::Label {
                            text: "Platform-Agnostic UI System (v1.0)".to_string(),
                            style: Style::from_tailwind("text-muted text-[12]"),
                        },
                    ],
                },
                Element::Container {
                    style: Style::from_tailwind("bg-header rounded-2xl p-10 col items-start gap-8"),
                    children: vec![
                        Element::Button {
                            id: ButtonId::Settings,
                            title: "Settings".to_string(),
                            style: Style::from_tailwind(&format!(
                                "{settings_bg} rounded-lg px-14 py-20 shadow-card h-[86]"
                            )),
                            hovered: state.hovered == Some(ButtonId::Settings),
                        },
                        Element::Button {
                            id: ButtonId::Contacts,
                            title: "Contacts".to_string(),
                            style: Style::from_tailwind(&format!(
                                "{contacts_bg} rounded-lg px-14 py-20 shadow-card h-[86]"
                            )),
                            hovered: state.hovered == Some(ButtonId::Contacts),
                        },
                        Element::Button {
                            id: ButtonId::Camera,
                            title: "Camera".to_string(),
                            style: Style::from_tailwind(&format!(
                                "{camera_bg} rounded-lg px-14 py-20 shadow-card h-[86]"
                            )),
                            hovered: state.hovered == Some(ButtonId::Camera),
                        },
                    ],
                },
            ],
        }
    }
}
