use crate::core::layout::LayoutNode;
use crate::core::types::Element;
use crate::core::{Point, Rect};

/// Fast AABB (Axis-Aligned Bounding Box) frustum culling check against viewport bounds.
#[must_use]
pub fn is_aabb_visible(rect: Rect, viewport: Rect, margin_x: f32, margin_y: f32) -> bool {
    let min_x = viewport.x - margin_x;
    let max_x = viewport.x + viewport.width + margin_x;
    let min_y = viewport.y - margin_y;
    let max_y = viewport.y + viewport.height + margin_y;

    rect.x + rect.width >= min_x
        && rect.x <= max_x
        && rect.y + rect.height >= min_y
        && rect.y <= max_y
}

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

#[allow(dead_code)]
pub(crate) fn shift_layout(layout: &mut LayoutNode, dx: f32, dy: f32) {
    layout.rect.x += dx;
    layout.rect.y += dy;
    for child in &mut layout.children {
        shift_layout(child, dx, dy);
    }
}
