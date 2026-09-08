use sniffer_core::layout::LayoutNode;
use sniffer_core::types::Element;
use sniffer_core::{Point, Rect};

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

/// Recursively traverses the layout and element trees to find which widget was clicked, supporting dynamic active scroll overrides.
#[must_use]
pub fn find_clicked_button_with_scroll(
    element: &Element,
    layout: &LayoutNode,
    point: Point,
    get_active_scroll: &dyn Fn(Option<&str>, f32, f32) -> (f32, f32),
) -> Option<(u64, Rect)> {
    find_clicked_button_with_scroll_recursive(element, layout, point, 1.0, get_active_scroll)
}

fn find_clicked_button_with_scroll_recursive(
    element: &Element,
    layout: &LayoutNode,
    point: Point,
    parent_opacity: f32,
    get_active_scroll: &dyn Fn(Option<&str>, f32, f32) -> (f32, f32),
) -> Option<(u64, Rect)> {
    let effective_opacity = parent_opacity * element.style().opacity;
    if effective_opacity < 0.05 {
        return None;
    }

    let tx = element.style().transform.translate_x;
    let ty = element.style().transform.translate_y;
    let safe_tx = if tx.is_nan() || tx.is_infinite() {
        0.0
    } else {
        tx
    };
    let safe_ty = if ty.is_nan() || ty.is_infinite() {
        0.0
    } else {
        ty
    };
    let local_point = Point::new(point.x - safe_tx, point.y - safe_ty);

    if !layout.rect.contains(local_point) {
        return None;
    }

    let transformed_rect = Rect::new(
        layout.rect.x + safe_tx,
        layout.rect.y + safe_ty,
        layout.rect.width,
        layout.rect.height,
    );

    match element {
        Element::Container { children, id, .. } | Element::SharedView { children, id, .. } => {
            for (child_el, child_lay) in children.iter().zip(layout.children.iter()).rev() {
                if let Some(clicked_data) = find_clicked_button_with_scroll_recursive(
                    child_el,
                    child_lay,
                    local_point,
                    effective_opacity,
                    get_active_scroll,
                ) {
                    return Some((
                        clicked_data.0,
                        Rect::new(
                            clicked_data.1.x + safe_tx,
                            clicked_data.1.y + safe_ty,
                            clicked_data.1.width,
                            clicked_data.1.height,
                        ),
                    ));
                }
            }
            if let Some(id_str) = id {
                if !sniffer_core::ui::widget::is_structural_layout_id(id_str) {
                    return Some((
                        sniffer_core::ui::widget::fnv1a(id_str.as_bytes()),
                        transformed_rect,
                    ));
                }
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
            let (active_x, active_y) = get_active_scroll(id.as_deref(), *scroll_x, *scroll_y);
            let offset_point = Point::new(local_point.x + active_x, local_point.y + active_y);
            for (child_el, child_lay) in children.iter().zip(layout.children.iter()).rev() {
                if let Some(clicked_data) = find_clicked_button_with_scroll_recursive(
                    child_el,
                    child_lay,
                    offset_point,
                    effective_opacity,
                    get_active_scroll,
                ) {
                    return Some((
                        clicked_data.0,
                        Rect::new(
                            clicked_data.1.x + safe_tx,
                            clicked_data.1.y + safe_ty,
                            clicked_data.1.width,
                            clicked_data.1.height,
                        ),
                    ));
                }
            }
            // Note: ScrollView is a scroll viewport container, not a clickable button target.
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
                    sniffer_core::ui::widget::fnv1a(id_str.as_bytes()),
                    transformed_rect,
                ));
            }
            None
        }
    }
}

/// Recursively traverses the layout and element trees to find which widget was clicked.
#[must_use]
pub fn find_clicked_button(
    element: &Element,
    layout: &LayoutNode,
    point: Point,
) -> Option<(u64, Rect)> {
    find_clicked_button_with_scroll(element, layout, point, &|_id, sx, sy| (sx, sy))
}

/// Recursively traverses the layout and element trees to find which widget is currently hovered, supporting dynamic active scroll overrides.
#[must_use]
pub fn find_hovered_button_with_scroll(
    element: &Element,
    layout: &LayoutNode,
    point: Point,
    get_active_scroll: &dyn Fn(Option<&str>, f32, f32) -> (f32, f32),
) -> Option<(u64, Rect)> {
    find_clicked_button_with_scroll(element, layout, point, get_active_scroll)
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
    find_hovered_scrollview_recursive(element, layout, point, 1.0)
}

fn find_hovered_scrollview_recursive<'a>(
    element: &'a Element,
    layout: &'a LayoutNode,
    point: Point,
    parent_opacity: f32,
) -> Option<(&'a Element, &'a LayoutNode)> {
    let effective_opacity = parent_opacity * element.style().opacity;
    if effective_opacity < 0.05 {
        return None;
    }

    let tx = element.style().transform.translate_x;
    let ty = element.style().transform.translate_y;
    let safe_tx = if tx.is_nan() || tx.is_infinite() {
        0.0
    } else {
        tx
    };
    let safe_ty = if ty.is_nan() || ty.is_infinite() {
        0.0
    } else {
        ty
    };
    let local_point = Point::new(point.x - safe_tx, point.y - safe_ty);

    if !layout.rect.contains(local_point) {
        return None;
    }

    match element {
        Element::Container { children, .. } | Element::SharedView { children, .. } => {
            for (child_el, child_lay) in children.iter().zip(layout.children.iter()).rev() {
                if let Some(scrollview_data) = find_hovered_scrollview_recursive(
                    child_el,
                    child_lay,
                    local_point,
                    effective_opacity,
                ) {
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
            let offset_point = Point::new(local_point.x + scroll_x, local_point.y + scroll_y);
            for (child_el, child_lay) in children.iter().zip(layout.children.iter()).rev() {
                if let Some(scrollview_data) = find_hovered_scrollview_recursive(
                    child_el,
                    child_lay,
                    offset_point,
                    effective_opacity,
                ) {
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
