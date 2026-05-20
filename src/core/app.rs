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
    safe_area_top: f32,
    safe_area_bottom: f32,
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
            safe_area_top: 0.0,
            safe_area_bottom: 0.0,
        };
        app.relayout(width, height);
        app
    }

    pub fn set_safe_area(&mut self, top: f32, bottom: f32) {
        self.safe_area_top = top.max(0.0);
        self.safe_area_bottom = bottom.max(0.0);
    }

    pub fn safe_area_top(&self) -> f32 {
        self.safe_area_top
    }

    pub fn safe_area_bottom(&self) -> f32 {
        self.safe_area_bottom
    }

    pub fn relayout(&mut self, width: usize, height: usize) {
        self.window = Size {
            width: width as f32,
            height: height as f32,
        };

        let button_width = self.window.width - (PANEL_PADDING * 2.0);
        let min_top = self.safe_area_top + PANEL_PADDING;
        let content_top = min_top + 96.0;
        let total_buttons_height = BUTTON_HEIGHT * 3.0 + BUTTON_GAP * 2.0;
        let bottom_limit =
            self.window.height - self.safe_area_bottom - PANEL_PADDING - total_buttons_height;
        let top = content_top.min(bottom_limit).max(min_top);

        self.buttons = vec![
            self.make_button(ButtonId::Settings, "Settings", top, button_width.max(0.0)),
            self.make_button(
                ButtonId::Contacts,
                "Contacts",
                top + BUTTON_HEIGHT + BUTTON_GAP,
                button_width.max(0.0),
            ),
            self.make_button(
                ButtonId::Camera,
                "Camera",
                top + ((BUTTON_HEIGHT + BUTTON_GAP) * 2.0),
                button_width.max(0.0),
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
