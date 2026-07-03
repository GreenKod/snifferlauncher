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
        Element::ScrollView {
            children,
            id,
            scroll_x,
            scroll_y,
            scroll_sensitivity: _,
            dynamic_sensitivity: _,
            momentum_scrolling: _,
            capture_drag: _,
            ..
        } => {
            let offset_point = Point::new(point.x + scroll_x, point.y + scroll_y);
            for (child_el, child_lay) in children.iter().zip(layout.children.iter()) {
                if let Some(clicked_data) = find_clicked_button(child_el, child_lay, offset_point) {
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
        Element::Label { id, .. }
        | Element::Image { id, .. }
        | Element::TextInput { id, .. }
        | Element::Checkbox { id, .. }
        | Element::Slider { id, .. }
        | Element::ProgressBar { id, .. } => {
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

/// Recursively traverses the layout and element trees to find the deepest ScrollView containing the point.
#[must_use]
pub fn find_hovered_scrollview<'a>(
    element: &'a Element,
    layout: &'a LayoutNode,
    point: Point,
) -> Option<(&'a Element, &'a LayoutNode)> {
    if !layout.rect.contains(point) {
        return None;
    }
    match element {
        Element::Container { children, .. } => {
            for (child_el, child_lay) in children.iter().zip(layout.children.iter()) {
                if let Some(scrollview_data) = find_hovered_scrollview(child_el, child_lay, point) {
                    return Some(scrollview_data);
                }
            }
            None
        }
        Element::ScrollView {
            children,
            scroll_x,
            scroll_y,
            scroll_sensitivity: _,
            dynamic_sensitivity: _,
            momentum_scrolling: _,
            capture_drag: _,
            ..
        } => {
            let offset_point = Point::new(point.x + scroll_x, point.y + scroll_y);
            for (child_el, child_lay) in children.iter().zip(layout.children.iter()) {
                if let Some(scrollview_data) =
                    find_hovered_scrollview(child_el, child_lay, offset_point)
                {
                    return Some(scrollview_data);
                }
            }
            Some((element, layout))
        }
        _ => None,
    }
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
    alpha_multiplier: f32,
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

    // Combine parent opacity with this element's opacity
    let current_alpha = alpha_multiplier * style.opacity;
    renderer.set_global_alpha(current_alpha);

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

    // 2. Draw background card / gradient
    if let Some((color_top, color_bottom)) = style.background_gradient {
        renderer.draw_rect_gradient(
            rect,
            color_top,
            color_bottom,
            style.border_radius,
            style.border_width,
            style.border_color,
        );
    } else if let Some(bg_color) = style.background_color {
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
                draw_ui(
                    renderer,
                    child_el,
                    child_lay,
                    metrics,
                    style_map,
                    data_map,
                    current_alpha,
                );
            }
        }
        Element::ScrollView {
            children,
            scroll_x,
            scroll_y,
            scroll_sensitivity: _,
            dynamic_sensitivity: _,
            momentum_scrolling: _,
            capture_drag: _,
            ..
        } => {
            renderer.set_clip_rect(rect);
            let mut max_y = 0.0_f32;
            for (child_el, child_lay) in children.iter().zip(layout.children.iter()) {
                let mut offset_lay = child_lay.clone();

                // Calculate max content height for the scrollbar
                let child_bottom = child_lay.rect.y + child_lay.rect.height;
                if child_bottom > max_y {
                    max_y = child_bottom;
                }

                // Shift rects recursively so children draw with scroll offset
                shift_layout(&mut offset_lay, -*scroll_x, -*scroll_y);
                draw_ui(
                    renderer,
                    child_el,
                    &offset_lay,
                    metrics,
                    style_map,
                    data_map,
                    current_alpha,
                );
            }
            renderer.clear_clip_rect();

            // Draw visual scrollbar if content exceeds container
            let content_height = max_y - rect.y;
            if content_height > rect.height {
                renderer.set_global_alpha(current_alpha * 0.5); // Semi-transparent scrollbar
                let ratio = rect.height / content_height;
                let scrollbar_height = (rect.height * ratio).max(20.0);

                // Max scroll distance
                let max_scroll = content_height - rect.height;
                let scroll_pct = (scroll_y / max_scroll).clamp(0.0, 1.0);

                let scrollbar_y = rect.y + (rect.height - scrollbar_height) * scroll_pct;
                let scrollbar_rect = Rect::new(
                    rect.x + rect.width - 6.0,
                    scrollbar_y,
                    4.0,
                    scrollbar_height,
                );

                renderer.draw_rect(scrollbar_rect, 0xFF00_0000, 2.0, 0.0, None);
                renderer.set_global_alpha(current_alpha); // Restore
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
        Element::Checkbox { checked, .. } => {
            if *checked {
                let inner_rect = Rect::new(
                    rect.x + 4.0,
                    rect.y + 4.0,
                    rect.width - 8.0,
                    rect.height - 8.0,
                );
                renderer.draw_rect(
                    inner_rect,
                    style.text_color.unwrap_or(0xFF00_0000),
                    2.0,
                    0.0,
                    None,
                );
            }
        }
        Element::Slider {
            value, min, max, ..
        } => {
            let track_rect = Rect::new(rect.x, rect.y + rect.height / 2.0 - 2.0, rect.width, 4.0);
            renderer.draw_rect(track_rect, 0xFF88_8888, 2.0, 0.0, None);

            let range = (max - min).max(0.0001);
            let pct = ((value - min) / range).clamp(0.0, 1.0);
            let thumb_x = rect.x + (rect.width - 16.0) * pct;
            let thumb_rect = Rect::new(thumb_x, rect.y + rect.height / 2.0 - 8.0, 16.0, 16.0);
            renderer.draw_rect(
                thumb_rect,
                style.text_color.unwrap_or(0xFF00_0000),
                8.0,
                0.0,
                None,
            );
        }
        Element::ProgressBar { value, max, .. } => {
            let pct = (value / max.max(0.0001)).clamp(0.0, 1.0);
            let fill_rect = Rect::new(rect.x, rect.y, rect.width * pct, rect.height);
            renderer.draw_rect(
                fill_rect,
                style.text_color.unwrap_or(0xFF00_00FF),
                style.border_radius,
                0.0,
                None,
            );
        }
    }

    if style.overflow_hidden {
        renderer.clear_clip_rect();
    }

    // Restore parent alpha multiplier
    renderer.set_global_alpha(alpha_multiplier);
}

fn shift_layout(layout: &mut LayoutNode, dx: f32, dy: f32) {
    layout.rect.x += dx;
    layout.rect.y += dy;
    for child in &mut layout.children {
        shift_layout(child, dx, dy);
    }
}
