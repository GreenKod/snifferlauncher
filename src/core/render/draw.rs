use crate::core::layout::LayoutNode;
use crate::core::render::api::Renderer;
use crate::core::style::{BUTTON_MUTED, BUTTON_TEXT, ICON_SURFACE};
use crate::core::types::{ButtonId, Element};
use crate::core::{Point, Rect, ScreenMetrics};

/// Recursively traverses the layout and element trees to find which button was clicked.
#[must_use]
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

/// Recursively traverses the layout and element trees to find which button is currently hovered.
#[must_use]
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

/// Platform-agnostic traversal to draw the UI elements using the Renderer interface.
pub fn draw_ui(
    renderer: &mut dyn Renderer,
    element: &Element,
    layout: &LayoutNode,
    metrics: &ScreenMetrics,
) {
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
                draw_ui(renderer, child_el, child_lay, metrics);
            }
        }
        Element::Button { id, title, .. } => {
            // Sub-elements of the Button:
            // Calculate proportions based on the actual height of the button (which scales by breakpoint)
            let icon_size = rect.height * 0.65;
            let icon_padding_left = rect.height * 0.16;

            // Left Icon Box
            let icon_box = Rect::new(
                rect.x + icon_padding_left,
                (rect.height - icon_size).mul_add(0.5, rect.y),
                icon_size,
                icon_size,
            );
            renderer.draw_rect(icon_box, ICON_SURFACE, icon_size * 0.28, 0.0, None);
            draw_button_icon(renderer, *id, icon_box, metrics);

            // Left Title / Description text
            let text_left = rect.height.mul_add(0.2, icon_box.x + icon_box.width);
            let title_size = rect.height * 0.18; // ~16sp at 86dp
            let subtitle_size = rect.height * 0.11; // ~10sp at 86dp

            // Center texts vertically
            let text_gap = rect.height * 0.09;
            let total_text_height = title_size + text_gap + subtitle_size;
            let text_start_y = (rect.height - total_text_height).mul_add(0.5, rect.y);

            renderer.draw_text(title, text_left, text_start_y, title_size, BUTTON_TEXT);
            renderer.draw_text(
                "Tap to launch application",
                text_left,
                text_start_y + title_size + text_gap,
                subtitle_size,
                BUTTON_MUTED,
            );

            // Right Accent Pill
            let pill_height = rect.height * 0.28;
            let pill_width = pill_height * 1.5;
            let pill_padding_right = rect.height * 0.16;
            let pill = Rect::new(
                rect.x + rect.width - pill_padding_right - pill_width,
                (rect.height - pill_height).mul_add(0.5, rect.y),
                pill_width,
                pill_height,
            );
            renderer.draw_rect(pill, BUTTON_TEXT, pill_height * 0.5, 0.0, None);
        }
        Element::Label { text, .. } => {
            let color = style.text_color.unwrap_or(BUTTON_TEXT);
            renderer.draw_text(text, rect.x, rect.y, style.text_size, color);
        }
        Element::Icon { id, .. } => {
            draw_button_icon(renderer, *id, rect, metrics);
        }
    }
}

/// Helper to draw vector shapes representing icons for Settings, Contacts, Camera.
pub fn draw_button_icon(
    renderer: &mut dyn Renderer,
    id: ButtonId,
    rect: Rect,
    _metrics: &ScreenMetrics,
) {
    // We use proportional math based on the provided rect bounds
    // so it scales automatically with the icon_box size, regardless of density or breakpoint.
    let w = rect.width;
    let h = rect.height;

    match id {
        ButtonId::Settings => {
            // Draws slider controls
            for (index, y_ratio) in [0.25f32, 0.50, 0.75].into_iter().enumerate() {
                let track_w = w * 0.65;
                let track_h = h * 0.07;
                let track_x = (w - track_w).mul_add(0.5, rect.x); // Centered
                let track_y = h.mul_add(y_ratio, rect.y) - (track_h * 0.5);

                renderer.draw_rect(
                    Rect::new(track_x, track_y, track_w, track_h),
                    BUTTON_MUTED,
                    track_h * 0.5,
                    0.0,
                    None,
                );

                let knob_r = h * 0.11;
                let knob_center_x = if index % 2 == 0 {
                    track_w.mul_add(0.25, track_x)
                } else {
                    track_w.mul_add(0.75, track_x)
                };
                let knob_center_y = track_h.mul_add(0.5, track_y);
                renderer.draw_circle(knob_center_x, knob_center_y, knob_r, BUTTON_TEXT);
            }
        }
        ButtonId::Contacts => {
            // Draw person avatar
            let head_r = h * 0.18;
            let head_center_x = w.mul_add(0.5, rect.x);
            let head_center_y = h.mul_add(0.35, rect.y);
            renderer.draw_circle(head_center_x, head_center_y, head_r, BUTTON_TEXT);

            let body_w = w * 0.5;
            let body_h = h * 0.22;
            let body_x = (w - body_w).mul_add(0.5, rect.x);
            let body_y = h.mul_add(0.65, rect.y);
            renderer.draw_rect(
                Rect::new(body_x, body_y, body_w, body_h),
                BUTTON_TEXT,
                body_h * 0.5,
                0.0,
                None,
            );
        }
        ButtonId::Camera => {
            // Draw camera body, lens, flash
            let body_w = w * 0.6;
            let body_h = h * 0.45;
            let body_x = (w - body_w).mul_add(0.5, rect.x);
            // slightly shifted down from center
            let body_y = h.mul_add(0.05, (h - body_h).mul_add(0.5, rect.y));
            renderer.draw_rect(
                Rect::new(body_x, body_y, body_w, body_h),
                BUTTON_TEXT,
                body_h * 0.2,
                0.0,
                None,
            );

            let lens_r = h * 0.125;
            let lens_center_x = w.mul_add(0.5, rect.x);
            let lens_center_y = body_h.mul_add(0.5, body_y);
            renderer.draw_circle(lens_center_x, lens_center_y, lens_r, ICON_SURFACE);

            let flash_w = w * 0.18;
            let flash_h = h * 0.11;
            let flash_x = body_w.mul_add(0.15, body_x);
            let flash_y = body_y - flash_h * 0.5;
            renderer.draw_rect(
                Rect::new(flash_x, flash_y, flash_w, flash_h),
                BUTTON_TEXT,
                flash_h * 0.5,
                0.0,
                None,
            );
        }
    }
}
