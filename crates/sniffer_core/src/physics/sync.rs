use super::scroll::ScrollPhysics;
use crate::layout::LayoutNode;
use crate::types::Element;
use crate::ui::widget::fnv1a;
use std::collections::HashMap;
use std::hash::BuildHasher;

pub fn sync_scroll_physics_from_tree<S: BuildHasher>(
    element: &Element,
    physics: &mut HashMap<u64, ScrollPhysics, S>,
) {
    if let Element::ScrollView {
        id: Some(id_str),
        snap_x,
        snap_y,
        rubber_band,
        page_count,
        on_snap,
        ..
    } = element
    {
        let wid = fnv1a(id_str.as_bytes());
        let entry = physics.entry(wid).or_default();

        // If snap_x changed (e.g. viewport resize), reposition to the correct page.
        if let (Some(new_snap_x), Some(old_snap_x)) = (*snap_x, entry.snap_x) {
            if (new_snap_x - old_snap_x).abs() > 0.5 && !entry.is_dragging {
                let max_page = (entry.page_count.unwrap_or(1) as f32 - 1.0).max(0.0);
                let clamped_page = (entry.last_snap_page as f32).clamp(0.0, max_page);
                entry.pos_x = clamped_page * new_snap_x;
                entry.vel_x = 0.0;
                entry.vel_y = 0.0;
                entry.snap_target_x = None;
                entry.spring_sim_x = None;
                entry.spring_sim_y = None;
            }
        }

        // Always sync snap/rubber-band config (even when snap_x/snap_y are None).
        entry.snap_x = *snap_x;
        entry.rubber_band = *rubber_band;
        entry.page_count = *page_count;
        entry.on_snap.clone_from(on_snap);
        let _ = snap_y; // snap_y is tracked via UiEvent::Scroll, not physics
    }

    match element {
        Element::Container { children, .. }
        | Element::ScrollView { children, .. }
        | Element::SharedView { children, .. } => {
            for child in children {
                sync_scroll_physics_from_tree(child, physics);
            }
        }
        _ => {}
    }
}

pub fn inject_physics_to_tree(element: &mut Element, target_id: u64, pos_x: f32, pos_y: f32) {
    if let Element::ScrollView {
        id: Some(id_str),
        scroll_x,
        scroll_y,
        ..
    } = element
    {
        if fnv1a(id_str.as_bytes()) == target_id {
            *scroll_x = pos_x;
            *scroll_y = pos_y;
            return;
        }
    }
    match element {
        Element::Container { children, .. }
        | Element::ScrollView { children, .. }
        | Element::SharedView { children, .. } => {
            for child in children {
                inject_physics_to_tree(child, target_id, pos_x, pos_y);
            }
        }
        _ => {}
    }
}

pub fn sync_max_scroll_from_layout<S: BuildHasher>(
    element: &Element,
    layout: &LayoutNode,
    physics: &mut HashMap<u64, ScrollPhysics, S>,
) {
    let mut stack = vec![(element, layout)];
    while let Some((el, lay)) = stack.pop() {
        if let Element::ScrollView {
            id: Some(id_str), ..
        } = el
        {
            let sv_id = fnv1a(id_str.as_bytes());
            if let Some(phys) = physics.get_mut(&sv_id) {
                let mut min_y = f32::MAX;
                let mut max_y = f32::MIN;
                for child in &lay.children {
                    if child.rect.y < min_y {
                        min_y = child.rect.y;
                    }
                    let bottom = child.rect.y + child.rect.height;
                    if bottom > max_y {
                        max_y = bottom;
                    }
                }
                let content_height = if min_y <= max_y { max_y - min_y } else { 0.0 };
                let max_scroll_y = (content_height - lay.rect.height).max(0.0);
                phys.max_y = Some(max_scroll_y);
            }
        }
        match el {
            Element::Container { children, .. }
            | Element::ScrollView { children, .. }
            | Element::SharedView { children, .. } => {
                for (c_el, c_lay) in children.iter().zip(lay.children.iter()) {
                    stack.push((c_el, c_lay));
                }
            }
            _ => {}
        }
    }
}
