use crate::core::geometry::{Point, Rect};
use crate::core::layout::LayoutNode;
use crate::core::renderer::Renderer;
use crate::core::style::Style;

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
pub enum Element {
    Container {
        style: Style,
        children: Vec<Element>,
    },
    Button {
        id: ButtonId,
        title: String,
        style: Style,
        hovered: bool,
    },
    Label {
        text: String,
        style: Style,
    },
    Icon {
        id: ButtonId,
        style: Style,
    },
}

impl Element {
    pub fn style(&self) -> &Style {
        match self {
            Element::Container { style, .. } => style,
            Element::Button { style, .. } => style,
            Element::Label { style, .. } => style,
            Element::Icon { style, .. } => style,
        }
    }
}

pub trait Application {
    type Message: Clone + std::fmt::Debug;
    type State: Default;

    fn update(state: &mut Self::State, msg: Self::Message) -> Option<Action>;
    fn view(state: &Self::State) -> Element;
}

/// Recursively traverses the layout and element trees to find which button was clicked
pub fn find_clicked_button(
    element: &Element,
    layout: &LayoutNode,
    point: Point,
) -> Option<ButtonId> {
    if !layout.rect.contains(point) {
        return None;
    }
    match element {
        Element::Button { id, .. } => Some(*id),
        Element::Container { children, .. } => {
            for (child_el, child_lay) in children.iter().zip(layout.children.iter()) {
                if let Some(clicked_id) = find_clicked_button(child_el, child_lay, point) {
                    return Some(clicked_id);
                }
            }
            None
        }
        _ => None,
    }
}

/// Recursively traverses the layout and element trees to find which button is currently hovered
pub fn find_hovered_button(
    element: &Element,
    layout: &LayoutNode,
    point: Point,
) -> Option<ButtonId> {
    if !layout.rect.contains(point) {
        return None;
    }
    match element {
        Element::Button { id, .. } => Some(*id),
        Element::Container { children, .. } => {
            for (child_el, child_lay) in children.iter().zip(layout.children.iter()) {
                if let Some(hovered_id) = find_hovered_button(child_el, child_lay, point) {
                    return Some(hovered_id);
                }
            }
            None
        }
        _ => None,
    }
}

/// Platform-agnostic traversal to draw the UI elements using the Renderer interface
pub fn draw_ui(renderer: &mut dyn Renderer, element: &Element, layout: &LayoutNode) {
    let rect = layout.rect;
    let style = element.style();

    // 1. Draw drop shadow
    if let Some(shadow_color) = style.shadow_color {
        renderer.draw_shadow(
            rect,
            style.border_radius,
            style.shadow_offset_y,
            style.shadow_spread,
            shadow_color,
        );
    }

    // 2. Draw background card
    if let Some(bg_color) = style.background_color {
        renderer.draw_rect(
            rect,
            bg_color,
            style.border_radius,
            style.border_width,
            style.border_color,
        );
    }

    // 3. Draw contents
    match element {
        Element::Container { children, .. } => {
            for (child_el, child_lay) in children.iter().zip(layout.children.iter()) {
                draw_ui(renderer, child_el, child_lay);
            }
        }
        Element::Button { id, title, .. } => {
            use crate::core::style::ICON_SURFACE;

            // Sub-elements of the Button:
            // Left Icon Box
            let icon_box = Rect::new(
                rect.x + 14.0,
                rect.y + (rect.height - 56.0) * 0.5,
                56.0,
                56.0,
            );
            renderer.draw_rect(icon_box, ICON_SURFACE, 16.0, 0.0, None);
            draw_button_icon(renderer, *id, icon_box);

            // Left Title / Description text
            let text_left = icon_box.x + icon_box.width + 18.0;
            renderer.draw_text(title, text_left, rect.y + 21.0, 16.0, 0xF2F5F7);
            renderer.draw_text(
                "Tap to launch application",
                text_left,
                rect.y + 45.0,
                10.0,
                0xAFC4CC,
            );

            // Right Accent Pill
            let pill = Rect::new(
                rect.x + rect.width - 50.0,
                rect.y + (rect.height - 24.0) * 0.5,
                36.0,
                24.0,
            );
            renderer.draw_rect(pill, 0xF2F5F7, 12.0, 0.0, None);
        }
        Element::Label { text, .. } => {
            let color = style.text_color.unwrap_or(0xFFFFFF);
            renderer.draw_text(text, rect.x, rect.y, style.text_size, color);
        }
        Element::Icon { id, .. } => {
            draw_button_icon(renderer, *id, rect);
        }
    }
}

/// Helper to draw vector shapes representing icons for Settings, Contacts, Camera
pub fn draw_button_icon(renderer: &mut dyn Renderer, id: ButtonId, rect: Rect) {
    match id {
        ButtonId::Settings => {
            // Draws slider controls
            for (index, y) in [14.0f32, 28.0, 42.0].into_iter().enumerate() {
                let track = Rect::new(rect.x + 10.0, rect.y + y, rect.width - 20.0, 4.0);
                renderer.draw_rect(track, 0xAFC4CC, 2.0, 0.0, None);

                let knob_x = rect.x + if index % 2 == 0 { 16.0 } else { 30.0 };
                renderer.draw_circle(knob_x + 6.0, track.y + 2.0, 6.0, 0xF2F5F7);
            }
        }
        ButtonId::Contacts => {
            // Draw person avatar
            let head_cx = rect.x + rect.width * 0.5;
            let head_cy = rect.y + 22.0;
            renderer.draw_circle(head_cx, head_cy, 10.0, 0xF2F5F7);

            let body = Rect::new(rect.x + 14.0, rect.y + 36.0, 28.0, 12.0);
            renderer.draw_rect(body, 0xF2F5F7, 6.0, 0.0, None);
        }
        ButtonId::Camera => {
            // Draw camera body, lens, flash
            let body = Rect::new(rect.x + 11.0, rect.y + 16.0, 34.0, 24.0);
            renderer.draw_rect(body, 0xF2F5F7, 8.0, 0.0, None);

            let lens_cx = rect.x + 28.0;
            let lens_cy = rect.y + 28.0;
            renderer.draw_circle(lens_cx, lens_cy, 7.0, 0x0F3940);

            let flash = Rect::new(rect.x + 18.0, rect.y + 12.0, 10.0, 6.0);
            renderer.draw_rect(flash, 0xF2F5F7, 3.0, 0.0, None);
        }
    }
}
