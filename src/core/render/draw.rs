#![allow(clippy::pedantic, clippy::nursery, clippy::only_used_in_recursion)]
use crate::core::layout::LayoutNode;
use crate::core::render::api::Renderer;
use crate::core::style::BUTTON_TEXT;
use crate::core::types::Element;
use crate::core::ui::data_map::DataMap;
use crate::core::ui::style_map::StyleMap;
use crate::core::{Point, Rect, ScreenMetrics};

/// Recursively traverses the layout and element trees to find which widget was clicked.
#[must_use]
pub fn find_clicked_button(
    element: &Element,
    layout: &LayoutNode,
    point: Point,
) -> Option<(u64, Rect)> {
    if !layout.rect.contains(point) {
        return None;
    }
    match element {
        Element::Container { children, id, .. } => {
            // Check children first
            for (child_el, child_lay) in children.iter().zip(layout.children.iter()) {
                if let Some(clicked_data) = find_clicked_button(child_el, child_lay, point) {
                    return Some(clicked_data);
                }
            }
            if let Some(id_str) = id {
                return Some((
                    crate::core::ui::widget::fnv1a(id_str.as_bytes()),
                    layout.rect,
                ));
            }
            None
        }
        Element::Label { id, .. } | Element::Image { id, .. } | Element::TextInput { id, .. } => {
            if let Some(id_str) = id {
                return Some((
                    crate::core::ui::widget::fnv1a(id_str.as_bytes()),
                    layout.rect,
                ));
            }
            None
        }
    }
}

/// Recursively traverses the layout and element trees to find which widget is currently hovered.
#[must_use]
pub fn find_hovered_button(
    element: &Element,
    layout: &LayoutNode,
    point: Point,
) -> Option<(u64, Rect)> {
    // Exact same logic as click
    find_clicked_button(element, layout, point)
}

/// Platform-agnostic traversal to draw the UI elements using the Renderer interface.
#[allow(clippy::too_many_lines)]
pub fn draw_ui(
    renderer: &mut dyn Renderer,
    element: &Element,
    layout: &LayoutNode,
    metrics: &ScreenMetrics,
    style_map: &StyleMap,
    data_map: &DataMap,
) {
    use crate::core::ui::data_map::DataValue;
    use crate::core::ui::data_map::DrawCommand;
    let rect = layout.rect;

    let widget_id = element
        .id()
        .map(|id_str| crate::core::ui::widget::fnv1a(id_str.as_bytes()));

    // Use plugin-provided style override if available, fallback to element's own style
    let style = if let Some(w_id) = widget_id {
        style_map.get(w_id).map_or_else(
            || element.style().clone(),
            |override_style| override_style.apply(element.style().clone()),
        )
    } else {
        element.style().clone()
    };

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

    // 3. Draw custom Wasm plugin commands if this element has an ID
    if let Some(w_id) = widget_id
        && let Some(DataValue::DrawList(cmds)) = data_map.get(w_id, "draw_list")
    {
        for cmd in cmds {
            match cmd {
                DrawCommand::Rect {
                    x,
                    y,
                    w,
                    h,
                    color,
                    radius,
                } => {
                    let r = Rect::new(rect.x + x, rect.y + y, w, h);
                    renderer.draw_rect(r, color, radius, 0.0, None);
                }
                DrawCommand::Circle { cx, cy, r, color } => {
                    renderer.draw_circle(rect.x + cx, rect.y + cy, r, color);
                }
                DrawCommand::Text {
                    text,
                    x,
                    y,
                    size,
                    color,
                } => {
                    renderer.draw_text(&text, rect.x + x, rect.y + y, size, color);
                }
            }
        }
    }

    // 4. Handle clipping
    if style.overflow_hidden {
        renderer.set_clip_rect(rect);
    }

    // 5. Draw contents
    match element {
        Element::Container { children, .. } => {
            for (child_el, child_lay) in children.iter().zip(layout.children.iter()) {
                draw_ui(renderer, child_el, child_lay, metrics, style_map, data_map);
            }
        }
        Element::Label { text, .. } => {
            // Check if plugin overrides the label text
            let display_text = if let Some(w_id) = widget_id {
                data_map.label(w_id).unwrap_or_else(|| text.clone())
            } else {
                text.clone()
            };

            let color = style.text_color.unwrap_or(BUTTON_TEXT);
            let text_w = renderer.measure_text(&display_text, style.text_size);

            // Center horizontally and vertically within the node bounds
            let draw_x = rect.x + (rect.width - text_w) / 2.0;
            let draw_y = rect.y + (rect.height - style.text_size) / 2.0;

            renderer.draw_text(&display_text, draw_x, draw_y, style.text_size, color);
        }
        Element::Image { id, src, .. } => {
            let img_id = id.as_deref().unwrap_or(src.as_str());
            renderer.draw_image(img_id, rect, style.border_radius, style.object_fit);
        }
        Element::TextInput { value, focused, .. } => {
            // Draw text
            let display_text = if *focused {
                format!("{value}_")
            } else {
                value.clone()
            };
            let color = style.text_color.unwrap_or(BUTTON_TEXT);

            // Left align with padding, center vertically
            let draw_x = rect.x + style.padding.left;
            let draw_y = rect.y + (rect.height - style.text_size) / 2.0;

            renderer.draw_text(&display_text, draw_x, draw_y, style.text_size, color);
        }
    }

    if style.overflow_hidden {
        renderer.clear_clip_rect();
    }
}
