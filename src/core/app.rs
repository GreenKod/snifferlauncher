use crate::core::geometry::{Point, Rect, Size};
use crate::core::style::{BUTTON_GAP, BUTTON_HEIGHT, PANEL_PADDING};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ButtonId {
    Settings,
    Contacts,
    Camera,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Action {
    OpenSettings,
    OpenContacts,
    OpenCamera,
}

#[derive(Clone, Debug)]
pub struct Button {
    pub id: ButtonId,
    pub title: &'static str,
    pub rect: Rect,
}

#[derive(Debug)]
pub struct App {
    window: Size,
    buttons: Vec<Button>,
    hovered: Option<ButtonId>,
}

impl App {
    pub fn new(width: usize, height: usize) -> Self {
        let mut app = Self {
            window: Size {
                width: width as f32,
                height: height as f32,
            },
            buttons: Vec::new(),
            hovered: None,
        };
        app.relayout(width, height);
        app
    }

    pub fn relayout(&mut self, width: usize, height: usize) {
        self.window = Size {
            width: width as f32,
            height: height as f32,
        };

        let button_width = self.window.width - (PANEL_PADDING * 2.0);
        let top = PANEL_PADDING + 96.0;

        self.buttons = vec![
            self.make_button(ButtonId::Settings, "Settings", top, button_width),
            self.make_button(
                ButtonId::Contacts,
                "Contacts",
                top + BUTTON_HEIGHT + BUTTON_GAP,
                button_width,
            ),
            self.make_button(
                ButtonId::Camera,
                "Camera",
                top + ((BUTTON_HEIGHT + BUTTON_GAP) * 2.0),
                button_width,
            ),
        ];
    }

    pub fn buttons(&self) -> &[Button] {
        &self.buttons
    }

    pub fn hovered(&self) -> Option<ButtonId> {
        self.hovered
    }

    pub fn pointer_moved(&mut self, point: Point) {
        self.hovered = self
            .buttons
            .iter()
            .find(|button| button.rect.contains(point))
            .map(|button| button.id);
    }

    pub fn click(&self, point: Point) -> Option<Action> {
        let button = self.buttons.iter().find(|button| button.rect.contains(point))?;

        Some(match button.id {
            ButtonId::Settings => Action::OpenSettings,
            ButtonId::Contacts => Action::OpenContacts,
            ButtonId::Camera => Action::OpenCamera,
        })
    }

    fn make_button(&self, id: ButtonId, title: &'static str, y: f32, width: f32) -> Button {
        Button {
            id,
            title,
            rect: Rect {
                x: PANEL_PADDING,
                y,
                width,
                height: BUTTON_HEIGHT,
            },
        }
    }
}
