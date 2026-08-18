#![allow(clippy::pedantic, clippy::nursery, clippy::only_used_in_recursion)]

pub mod culling;
pub mod elements;

pub use culling::{
    find_clicked_button, find_clicked_button_with_scroll, find_hovered_button,
    find_hovered_button_with_scroll, find_hovered_scrollview, is_aabb_visible,
};

use sniffer_core::ScreenMetrics;
use sniffer_core::layout::LayoutNode;
use sniffer_core::render_api::Renderer;
use sniffer_core::types::Element;
use sniffer_core::ui::data_map::DataMap;
use sniffer_core::ui::style_map::StyleMap;

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
    transition_manager: &sniffer_core::anim::TransitionManager,
    alpha_multiplier: f32,
    accumulated_scroll_x: f32,
    accumulated_scroll_y: f32,
) -> usize {
    use sniffer_core::ui::data_map::DataValue;
    use sniffer_core::ui::data_map::DrawCommand;
    let rect = layout.rect;

    let widget_id = element
        .id()
        .map(|id_str| sniffer_core::ui::widget::fnv1a(id_str.as_bytes()));

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

    let mut tx = base_style.transform.translate_x;
    let mut ty = base_style.transform.translate_y;
    if tx.is_nan() || tx.is_infinite() {
        tx = 0.0;
    }
    if ty.is_nan() || ty.is_infinite() {
        ty = 0.0;
    }

    let screen_x = rect.x + tx - accumulated_scroll_x;
    let screen_y = rect.y + ty - accumulated_scroll_y;
    let eff_w = rect.width * base_style.transform.scale;
    let eff_h = rect.height * base_style.transform.scale;

    let is_animating = transition_manager.states.values().any(|s| s.is_active);

    let margin_x = 20.0_f32;
    let margin_y = 20.0_f32;
    let node_rect = sniffer_core::Rect::new(screen_x, screen_y, eff_w, eff_h);
    let viewport_rect = sniffer_core::Rect::new(0.0, 0.0, screen_w, screen_h);
    let is_offscreen = if is_animating {
        false
    } else {
        screen_w > 0.0
            && screen_h > 0.0
            && !is_aabb_visible(node_rect, viewport_rect, margin_x, margin_y)
    };

    if is_offscreen {
        return 0;
    }

    renderer.push_transform(
        rect.x + rect.width / 2.0,
        rect.y + rect.height / 2.0,
        base_style.transform.scale,
        base_style.transform.rotate,
        tx,
        ty,
    );

    renderer.set_global_alpha(final_alpha);

    if let Some(shadow_color) = base_style.shadow_color {
        renderer.draw_shadow(
            rect,
            base_style.border_radius,
            base_style.shadow_offset_y,
            base_style.shadow_spread,
            shadow_color,
        );
    }

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
                    let r = sniffer_core::Rect::new(rect.x + x, rect.y + y, w, h);
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

    if base_style.overflow_hidden {
        renderer.push_clip_rect(rect, base_style.border_radius);
    }

    let children_count = elements::draw_element_contents(
        renderer,
        element,
        layout,
        metrics,
        style_map,
        data_map,
        transition_manager,
        final_alpha,
        accumulated_scroll_x,
        accumulated_scroll_y,
        &base_style,
        rect,
        widget_id,
    );

    if base_style.overflow_hidden {
        renderer.pop_clip_rect();
    }

    renderer.set_global_alpha(alpha_multiplier);
    renderer.pop_transform();

    1 + children_count
}
