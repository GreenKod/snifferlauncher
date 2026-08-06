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
        Element::Container { children, id, .. }
        | Element::SharedView { children, id, .. } => {
            // Check children first (reverse order for z-index correctness)
            for (child_el, child_lay) in children.iter().zip(layout.children.iter()).rev() {
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
            ..
        } => {
            let offset_point = Point::new(point.x + scroll_x, point.y + scroll_y);
            for (child_el, child_lay) in children.iter().zip(layout.children.iter()).rev() {
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
        Element::Container { children, .. } | Element::SharedView { children, .. } => {
            for (child_el, child_lay) in children.iter().zip(layout.children.iter()).rev() {
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
            ..
        } => {
            let offset_point = Point::new(point.x + scroll_x, point.y + scroll_y);
            for (child_el, child_lay) in children.iter().zip(layout.children.iter()).rev() {
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
#[allow(clippy::too_many_arguments)]
pub fn draw_ui(
    renderer: &mut dyn Renderer,
    element: &Element,
    layout: &LayoutNode,
    metrics: &ScreenMetrics,
    style_map: &StyleMap,
    data_map: &DataMap,
    transition_manager: &crate::core::anim::TransitionManager,
    alpha_multiplier: f32,
    accumulated_scroll_x: f32,
    accumulated_scroll_y: f32,
) {
    use crate::core::ui::data_map::DataValue;
    use crate::core::ui::data_map::DrawCommand;
    let rect = layout.rect;

    let widget_id = element
        .id()
        .map(|id_str| crate::core::ui::widget::fnv1a(id_str.as_bytes()));

    let mut base_style = element.style().clone();
    if let Some(id_str) = element.id()
        && let Some(anim_style) = transition_manager.get_current_style(id_str)
    {
        base_style = anim_style.clone();
    }

    if let Some(id) = widget_id
        && let Some(override_style) = style_map.get(id)
    {
        base_style = override_style.apply(base_style);
    }

    let final_alpha = alpha_multiplier * base_style.opacity.clamp(0.0, 1.0);

    let screen_w = metrics.physical_width;
    let screen_h = metrics.physical_height;

    // Sanitize transform offsets against NaN / Infinity
    let mut tx = base_style.transform.translate_x;
    let mut ty = base_style.transform.translate_y;
    if tx.is_nan() || tx.is_infinite() { tx = 0.0; }
    if ty.is_nan() || ty.is_infinite() { ty = 0.0; }

    // ScrollView-aware effective screen bounds calculation
    let screen_x = rect.x + tx - accumulated_scroll_x;
    let screen_y = rect.y + ty - accumulated_scroll_y;
    let eff_w = rect.width * base_style.transform.scale;
    let eff_h = rect.height * base_style.transform.scale;

    let is_animating = transition_manager.states.values().any(|s| s.is_active);

    // Prefetch margin: margin_x expands to cover adjacent horizontal pages so cards render and scroll smoothly
    let margin_x = (screen_w * 3.0).max(2000.0);
    let margin_y = 200.0_f32;
    let is_offscreen = if is_animating {
        false
    } else {
        screen_w > 0.0 && screen_h > 0.0 && (
            screen_x + eff_w < -margin_x
                || screen_x > screen_w + margin_x
                || screen_y + eff_h < -margin_y
                || screen_y > screen_h + margin_y
        )
    };

    if is_offscreen {
        return;
    }

    // Apply transforms (scale, rotate, translate)
    renderer.push_transform(
        rect.x + rect.width / 2.0,
        rect.y + rect.height / 2.0,
        base_style.transform.scale,
        base_style.transform.rotate,
        tx,
        ty,
    );

    renderer.set_global_alpha(final_alpha);

    // 1. Draw drop shadow
    if let Some(shadow_color) = base_style.shadow_color {
        renderer.draw_shadow(
            rect,
            base_style.border_radius,
            base_style.shadow_offset_y,
            base_style.shadow_spread,
            shadow_color,
        );
    }

    // 2. Draw background card / gradient
    if let Some((color_top, color_bottom)) = base_style.background_gradient {
        renderer.draw_rect_gradient(
            rect,
            color_top,
            color_bottom,
            base_style.border_radius,
            base_style.border_width,
            base_style.border_color,
        );
    } else if let Some(bg_color) = base_style.background_color {
        renderer.draw_rect(
            rect,
            bg_color,
            base_style.border_radius,
            base_style.border_width,
            base_style.border_color,
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
    if base_style.overflow_hidden {
        renderer.push_clip_rect(rect, base_style.border_radius);
    }

    // 5. Draw contents
    match element {
        Element::Container { children, .. } | Element::SharedView { children, .. } => {
            for (child_el, child_lay) in children.iter().zip(layout.children.iter()) {
                draw_ui(
                    renderer,
                    child_el,
                    child_lay,
                    metrics,
                    style_map,
                    data_map,
                    transition_manager,
                    final_alpha,
                    accumulated_scroll_x,
                    accumulated_scroll_y,
                );
            }
        }
        Element::ScrollView {
            children,
            scroll_x,
            scroll_y,
            ..
        } => {
            renderer.push_clip_rect(rect, base_style.border_radius);

            // Correct content_height: span of child rects (not offset from rect.y)
            let mut min_y = f32::MAX;
            let mut max_y = f32::MIN;
            for child_lay in layout.children.iter() {
                if child_lay.rect.y < min_y { min_y = child_lay.rect.y; }
                let bottom = child_lay.rect.y + child_lay.rect.height;
                if bottom > max_y { max_y = bottom; }
            }
            let content_height = if min_y <= max_y { max_y - min_y } else { 0.0 };
            let max_scroll_y = (content_height - rect.height).max(0.0);
            // Sanitize scroll_y against NaN/Infinity before clamping
            let safe_scroll_y = if scroll_y.is_nan() || scroll_y.is_infinite() { 0.0 } else { *scroll_y };
            let actual_scroll_y = safe_scroll_y.clamp(0.0, max_scroll_y);
            let safe_scroll_x = if scroll_x.is_nan() || scroll_x.is_infinite() { 0.0 } else { *scroll_x };
            let actual_scroll_x = safe_scroll_x.max(0.0);


            renderer.push_transform(0.0, 0.0, 1.0, 0.0, -actual_scroll_x, -actual_scroll_y);
            for (child_el, child_lay) in children.iter().zip(layout.children.iter()) {
                draw_ui(
                    renderer,
                    child_el,
                    child_lay,
                    metrics,
                    style_map,
                    data_map,
                    transition_manager,
                    final_alpha,
                    accumulated_scroll_x + actual_scroll_x,
                    accumulated_scroll_y + actual_scroll_y,
                );
            }
            renderer.pop_transform();
            renderer.pop_clip_rect();

            // Draw visual scrollbar if content exceeds container
            if content_height > rect.height {
                renderer.set_global_alpha(final_alpha * 0.5); // Semi-transparent scrollbar
                let ratio = rect.height / content_height;
                let scrollbar_height = (rect.height * ratio).max(20.0);

                // Max scroll distance
                let max_scroll = content_height - rect.height;
                let scroll_pct = (safe_scroll_y / max_scroll).clamp(0.0, 1.0);

                let scrollbar_y = rect.y + (rect.height - scrollbar_height) * scroll_pct;
                let scrollbar_rect = Rect::new(
                    rect.x + rect.width - 6.0,
                    scrollbar_y,
                    4.0,
                    scrollbar_height,
                );

                renderer.draw_rect(scrollbar_rect, 0xFF00_0000, 2.0, 0.0, None);
                renderer.set_global_alpha(final_alpha); // Restore
            }
        }
        Element::Label { text, .. } => {
            // Check if plugin overrides the label text
            let display_text = if let Some(w_id) = widget_id {
                data_map.label(w_id).unwrap_or_else(|| text.clone())
            } else {
                text.clone()
            };

            let color = base_style.text_color.unwrap_or(BUTTON_TEXT);
            let text_w = renderer.measure_text(&display_text, base_style.text_size);

            // Center horizontally and vertically within the node bounds
            let draw_x = rect.x + (rect.width - text_w) / 2.0;
            let draw_y = rect.y + (rect.height - base_style.text_size) / 2.0;

            renderer.draw_text(&display_text, draw_x, draw_y, base_style.text_size, color);
        }
        Element::Image { id, src, .. } => {
            let img_id = id.as_deref().unwrap_or(src.as_str());

            if !renderer.has_image(img_id) {
                if let Some(pkg_name) = src.strip_prefix("app-icon://") {
                    #[cfg(target_os = "android")]
                    {
                        if let Some((pixels, w, h)) =
                            crate::platform::android::jni::bridge::get_app_icon_pixels(pkg_name)
                        {
                            renderer.load_image(img_id, &pixels, w, h);
                        } else {
                            // Insert a 1x1 transparent dummy texture to cache failure and prevent redundant JNI calls
                            let dummy_pixel = [0u8, 0u8, 0u8, 0u8];
                            renderer.load_image(img_id, &dummy_pixel, 1, 1);
                        }
                    }
                    #[cfg(not(target_os = "android"))]
                    {
                        let _ = pkg_name;
                        let dummy_pixel = [0u8, 0u8, 0u8, 0u8];
                        renderer.load_image(img_id, &dummy_pixel, 1, 1);
                    }
                }
            }

            renderer.draw_image(
                img_id,
                rect,
                base_style.border_radius,
                base_style.object_fit,
            );
        }
        Element::TextInput { value, focused, .. } => {
            // Clip to the box — the GPU scissor rect prevents any text from
            // escaping outside the TextInput border even if text_size is large.
            renderer.push_clip_rect(rect, base_style.border_radius);

            let display_text = if *focused {
                format!("{value}_")
            } else {
                value.clone()
            };
            let color = base_style.text_color.unwrap_or(BUTTON_TEXT);

            // Use the style's text_size directly (already computed as vmin in JS,
            // so it is fully responsive). The clip rect handles any rare overflow.
            let ts = base_style.text_size;

            // Vertically center the text within the node bounds, matching Label behavior.
            let draw_y = rect.y + (rect.height - ts) / 2.0;
            let draw_x = rect.x + base_style.padding.left;

            renderer.draw_text(&display_text, draw_x, draw_y, ts, color);

            renderer.pop_clip_rect();
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
                    base_style.text_color.unwrap_or(0xFF00_0000),
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
                base_style.text_color.unwrap_or(0xFF00_0000),
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
                base_style.text_color.unwrap_or(0xFF00_00FF),
                base_style.border_radius,
                0.0,
                None,
            );
        }
    }

    if base_style.overflow_hidden {
        renderer.pop_clip_rect();
    }

    // Restore parent alpha multiplier
    renderer.set_global_alpha(alpha_multiplier);
    renderer.pop_transform();
}

#[allow(dead_code)]
fn shift_layout(layout: &mut LayoutNode, dx: f32, dy: f32) {
    layout.rect.x += dx;
    layout.rect.y += dy;
    for child in &mut layout.children {
        shift_layout(child, dx, dy);
    }
}
