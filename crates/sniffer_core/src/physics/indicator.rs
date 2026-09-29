use crate::layout::LayoutNode;
use crate::style::FlexDirection;
use crate::types::Element;

#[allow(
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::many_single_char_names
)]
fn update_indicator_element_tree(
    element: &mut Element,
    page_float: f32,
    vmin_px: f32,
) -> Option<bool> {
    if let Element::Container {
        id,
        children,
        style,
        ..
    } = element
    {
        if let Some(id_str) = id {
            if id_str == "page_indicator_container" {
                let is_landscape = style.flex_direction == FlexDirection::Column;
                let mut changed = false;
                for (p, child_el) in children.iter_mut().enumerate() {
                    let dist = (page_float - p as f32).abs();
                    let raw_activity = (1.0 - dist).clamp(0.0, 1.0);
                    // Hermite smoothstep for fluid visual transition without linear harshness
                    let activity = raw_activity * raw_activity * (3.0 - 2.0 * raw_activity);

                    if let Element::Container {
                        style: dot_style, ..
                    } = child_el
                    {
                        // Smoothly interpolate color between inactive (#55FFFFFF) and active (#00E5FF)
                        let a = (0x55 as f32 + (0xFF - 0x55) as f32 * activity).round() as u32;
                        let r = (0xFF as f32 * (1.0 - activity)).round() as u32;
                        let g = (0xFF as f32 - (0xFF - 0xE5) as f32 * activity).round() as u32;
                        let b = 0xFF_u32;
                        let new_color = Some((a << 24) | (r << 16) | (g << 8) | b);

                        if dot_style.background_color != new_color {
                            dot_style.background_color = new_color;
                            changed = true;
                        }

                        if (dot_style.transform.scale - 1.0).abs() > 0.001 {
                            dot_style.transform.scale = 1.0;
                            changed = true;
                        }

                        let dot_pill = (1.6 + 2.0 * activity) * vmin_px;
                        let dot_base = 1.6 * vmin_px;
                        if is_landscape {
                            let new_w = crate::style::Dimension::Pixels(dot_base);
                            let new_h = crate::style::Dimension::Pixels(dot_pill);
                            if dot_style.width != new_w || dot_style.height != new_h {
                                dot_style.width = new_w;
                                dot_style.height = new_h;
                                changed = true;
                            }
                        } else {
                            let new_w = crate::style::Dimension::Pixels(dot_pill);
                            let new_h = crate::style::Dimension::Pixels(dot_base);
                            if dot_style.width != new_w || dot_style.height != new_h {
                                dot_style.width = new_w;
                                dot_style.height = new_h;
                                changed = true;
                            }
                        }
                    }
                }
                return Some(changed);
            }
        }
        for child in children {
            if let Some(changed) = update_indicator_element_tree(child, page_float, vmin_px) {
                return Some(changed);
            }
        }
    }
    None
}

#[allow(
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::many_single_char_names
)]
fn update_indicator_layout_tree(
    layout_node: &mut LayoutNode,
    page_float: f32,
    vmin_px: f32,
) -> bool {
    if layout_node.element.id() == Some("page_indicator_container") {
        let is_landscape = layout_node.element.style().flex_direction == FlexDirection::Column;
        let count = layout_node.children.len();
        if count == 0 {
            return false;
        }

        let gap = 1.2 * vmin_px;
        let dot_base = 1.6 * vmin_px;
        let mut layout_changed = false;

        if is_landscape {
            let mut total_h = 0.0_f32;
            let mut dot_heights = Vec::with_capacity(count);
            for p in 0..count {
                let dist = (page_float - p as f32).abs();
                let raw_activity = (1.0 - dist).clamp(0.0, 1.0);
                let activity = raw_activity * raw_activity * (3.0 - 2.0 * raw_activity);
                let h = (1.6 + 2.0 * activity) * vmin_px;
                dot_heights.push((h, activity));
                total_h += h;
            }
            total_h += (count.saturating_sub(1) as f32) * gap;

            let center_x = layout_node.rect.x + (layout_node.rect.width - dot_base) * 0.5;
            let mut cur_y = layout_node.rect.y + (layout_node.rect.height - total_h) * 0.5;
            for (p, child) in layout_node.children.iter_mut().enumerate() {
                let (h, activity) = dot_heights[p];
                if (child.rect.x - center_x).abs() > 0.05
                    || (child.rect.y - cur_y).abs() > 0.05
                    || (child.rect.width - dot_base).abs() > 0.05
                    || (child.rect.height - h).abs() > 0.05
                {
                    child.rect.x = center_x;
                    child.rect.y = cur_y;
                    child.rect.width = dot_base;
                    child.rect.height = h;
                    layout_changed = true;
                }
                cur_y += h + gap;

                let a = (0x55 as f32 + (0xFF - 0x55) as f32 * activity).round() as u32;
                let r = (0xFF as f32 * (1.0 - activity)).round() as u32;
                let g = (0xFF as f32 - (0xFF - 0xE5) as f32 * activity).round() as u32;
                let b = 0xFF_u32;
                let new_bg = Some((a << 24) | (r << 16) | (g << 8) | b);
                if let Element::Container { style, .. } = &mut child.element {
                    if style.background_color != new_bg {
                        style.background_color = new_bg;
                        layout_changed = true;
                    }
                    style.transform.scale = 1.0;
                }
            }
        } else {
            let mut total_w = 0.0_f32;
            let mut dot_widths = Vec::with_capacity(count);
            for p in 0..count {
                let dist = (page_float - p as f32).abs();
                let raw_activity = (1.0 - dist).clamp(0.0, 1.0);
                let activity = raw_activity * raw_activity * (3.0 - 2.0 * raw_activity);
                let w = (1.6 + 2.0 * activity) * vmin_px;
                dot_widths.push((w, activity));
                total_w += w;
            }
            total_w += (count.saturating_sub(1) as f32) * gap;

            let mut cur_x = layout_node.rect.x + (layout_node.rect.width - total_w) * 0.5;
            let center_y = layout_node.rect.y + (layout_node.rect.height - dot_base) * 0.5;
            for (p, child) in layout_node.children.iter_mut().enumerate() {
                let (w, activity) = dot_widths[p];
                if (child.rect.x - cur_x).abs() > 0.05
                    || (child.rect.y - center_y).abs() > 0.05
                    || (child.rect.width - w).abs() > 0.05
                    || (child.rect.height - dot_base).abs() > 0.05
                {
                    child.rect.x = cur_x;
                    child.rect.y = center_y;
                    child.rect.width = w;
                    child.rect.height = dot_base;
                    layout_changed = true;
                }
                cur_x += w + gap;

                let a = (0x55 as f32 + (0xFF - 0x55) as f32 * activity).round() as u32;
                let r = (0xFF as f32 * (1.0 - activity)).round() as u32;
                let g = (0xFF as f32 - (0xFF - 0xE5) as f32 * activity).round() as u32;
                let b = 0xFF_u32;
                let new_bg = Some((a << 24) | (r << 16) | (g << 8) | b);
                if let Element::Container { style, .. } = &mut child.element {
                    if style.background_color != new_bg {
                        style.background_color = new_bg;
                        layout_changed = true;
                    }
                    style.transform.scale = 1.0;
                }
            }
        }
        return layout_changed;
    }

    for child in &mut layout_node.children {
        if update_indicator_layout_tree(child, page_float, vmin_px) {
            return true;
        }
    }
    false
}

#[allow(
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
pub fn update_indicator_dots_tracking(
    element: &mut Element,
    mut layout_tree: Option<&mut LayoutNode>,
    page_float: f32,
    vmin_px: f32,
) -> bool {
    let mut changed = false;
    if let Some(c) = update_indicator_element_tree(element, page_float, vmin_px) {
        if c {
            changed = true;
        }
    }
    if let Some(ref mut layout_node) = layout_tree {
        if update_indicator_layout_tree(layout_node, page_float, vmin_px) {
            changed = true;
        }
    }
    changed
}

#[allow(clippy::cast_possible_wrap)]
pub fn update_indicator_dots_in_element(
    element: &mut Element,
    active_page: i32,
    vmin_px: f32,
) -> bool {
    update_indicator_dots_tracking(element, None, active_page as f32, vmin_px)
}
