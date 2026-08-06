use super::draw_ui;
use crate::core::layout::LayoutNode;
use crate::core::render::api::Renderer;
use crate::core::style::BUTTON_TEXT;
use crate::core::types::Element;
use crate::core::ui::data_map::DataMap;
use crate::core::ui::style_map::StyleMap;
use crate::core::{Rect, ScreenMetrics};

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(crate) fn draw_element_contents(
    renderer: &mut dyn Renderer,
    element: &Element,
    layout: &LayoutNode,
    metrics: &ScreenMetrics,
    style_map: &StyleMap,
    data_map: &DataMap,
    transition_manager: &crate::core::anim::TransitionManager,
    final_alpha: f32,
    accumulated_scroll_x: f32,
    accumulated_scroll_y: f32,
    base_style: &crate::core::style::Style,
    rect: Rect,
    widget_id: Option<u64>,
) {
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

            let mut min_y = f32::MAX;
            let mut max_y = f32::MIN;
            for child_lay in layout.children.iter() {
                if child_lay.rect.y < min_y { min_y = child_lay.rect.y; }
                let bottom = child_lay.rect.y + child_lay.rect.height;
                if bottom > max_y { max_y = bottom; }
            }
            let content_height = if min_y <= max_y { max_y - min_y } else { 0.0 };
            let max_scroll_y = (content_height - rect.height).max(0.0);
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

            if content_height > rect.height {
                renderer.set_global_alpha(final_alpha * 0.5);
                let ratio = rect.height / content_height;
                let scrollbar_height = (rect.height * ratio).max(20.0);

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
                renderer.set_global_alpha(final_alpha);
            }
        }
        Element::Label { text, .. } => {
            let display_text = if let Some(w_id) = widget_id {
                data_map.label(w_id).unwrap_or_else(|| text.clone())
            } else {
                text.clone()
            };

            let color = base_style.text_color.unwrap_or(BUTTON_TEXT);
            let text_w = renderer.measure_text(&display_text, base_style.text_size);

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
            renderer.push_clip_rect(rect, base_style.border_radius);

            let display_text = if *focused {
                format!("{value}_")
            } else {
                value.clone()
            };
            let color = base_style.text_color.unwrap_or(BUTTON_TEXT);
            let ts = base_style.text_size;

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
}
