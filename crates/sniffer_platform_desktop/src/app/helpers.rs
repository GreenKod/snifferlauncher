use sniffer_core::layout::LayoutNode;
use sniffer_core::math::Rect;
use sniffer_core::physics::ScrollPhysics;
use sniffer_core::types::Element;
use std::collections::HashMap;
use std::hash::BuildHasher;

pub fn collect_layout_rects<'a, S: BuildHasher>(
    node: &'a LayoutNode,
    map: &mut HashMap<&'a str, Rect, S>,
) {
    if let Some(id) = node.element.id() {
        map.insert(id, node.rect);
    }
    for child in &node.children {
        collect_layout_rects(child, map);
    }
}

pub fn find_first_scrollview<'a>(
    element: &'a Element,
    layout: &'a LayoutNode,
) -> Option<(&'a Element, &'a LayoutNode)> {
    if matches!(element, Element::ScrollView { .. }) {
        return Some((element, layout));
    }
    if let Element::Container { children, .. } | Element::SharedView { children, .. } = element {
        for (child_el, child_lay) in children.iter().zip(&layout.children) {
            if let Some(res) = find_first_scrollview(child_el, child_lay) {
                return Some(res);
            }
        }
    }
    None
}

pub fn resolve_active_scroll<S: BuildHasher>(
    scroll_physics: &HashMap<u64, ScrollPhysics, S>,
    id_opt: Option<&str>,
    sx: f32,
    sy: f32,
) -> (f32, f32) {
    if let Some(id_str) = id_opt {
        let wid = sniffer_core::ui::widget::fnv1a(id_str.as_bytes());
        if let Some(phys) = scroll_physics.get(&wid) {
            return (phys.pos_x, phys.pos_y);
        }
    }
    (sx, sy)
}

pub fn get_max_scroll_for_lay(lay: &LayoutNode) -> (f32, f32) {
    let mut min_x = 0.0f32;
    let mut max_x = 0.0f32;
    let mut min_y = 0.0f32;
    let mut max_y = 0.0f32;

    for child in &lay.children {
        if child.rect.x < min_x {
            min_x = child.rect.x;
        }
        if child.rect.x + child.rect.width > max_x {
            max_x = child.rect.x + child.rect.width;
        }
        if child.rect.y < min_y {
            min_y = child.rect.y;
        }
        if child.rect.y + child.rect.height > max_y {
            max_y = child.rect.y + child.rect.height;
        }
    }

    (
        (max_x - min_x - lay.rect.width).max(0.0),
        (max_y - min_y - lay.rect.height).max(0.0),
    )
}
