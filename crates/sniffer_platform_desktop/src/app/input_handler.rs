use super::helpers::{find_first_scrollview, get_max_scroll_for_lay, resolve_active_scroll};
use super::state::{AppState, FrameInputState, KineticScroll};
use sniffer_core::Point;
use sniffer_core::layout::LayoutNode;
use sniffer_core::types::Element;
use sniffer_core::ui::event::UiEvent;
use sniffer_render::draw::{find_clicked_button_with_scroll, find_hovered_button_with_scroll};

#[allow(
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)]
pub fn process_frame_inputs(
    app: &mut AppState,
    input: &FrameInputState,
    root_element: &Element,
    layout_tree: &LayoutNode,
    scaled_last_mouse_pos: Point,
    scaled_clicked_pos: Option<Point>,
    scale_x: f32,
    scale_y: f32,
) {
    if input.mouse_moved {
        let prev_hovered = app.hovered_btn;
        let hovered_data = find_hovered_button_with_scroll(
            root_element,
            layout_tree,
            scaled_last_mouse_pos,
            &|id_opt, sx, sy| resolve_active_scroll(&app.scroll_physics, id_opt, sx, sy),
        );
        app.hovered_btn = hovered_data.map(|(id, _)| id);

        match (prev_hovered, hovered_data) {
            (Some(prev), Some((current, rect))) if prev != current => {
                app.event_bus.push(UiEvent::HoverEnd(prev));
                app.event_bus.push(UiEvent::Hover(
                    current,
                    rect.width,
                    rect.height,
                    scaled_last_mouse_pos.x - rect.x,
                    scaled_last_mouse_pos.y - rect.y,
                ));
            }
            (Some(prev), Some((current, rect))) if prev == current => {
                app.event_bus.push(UiEvent::Hover(
                    current,
                    rect.width,
                    rect.height,
                    scaled_last_mouse_pos.x - rect.x,
                    scaled_last_mouse_pos.y - rect.y,
                ));
            }
            (None, Some((current, rect))) => {
                app.event_bus.push(UiEvent::Hover(
                    current,
                    rect.width,
                    rect.height,
                    scaled_last_mouse_pos.x - rect.x,
                    scaled_last_mouse_pos.y - rect.y,
                ));
            }
            (Some(prev), None) => {
                app.event_bus.push(UiEvent::HoverEnd(prev));
            }
            _ => {}
        }
    }

    if let Some(clicked_pt) = scaled_clicked_pos {
        app.mouse_down_pos = Some(clicked_pt);
        app.total_drag_dist = 0.0;
        app.drag_history.clear();

        if let Some((
            Element::ScrollView {
                id, capture_drag, ..
            },
            _,
        )) = sniffer_render::draw::find_hovered_scrollview(root_element, layout_tree, clicked_pt)
        {
            if capture_drag.unwrap_or(true) {
                app.active_scrollview_drag = id
                    .as_deref()
                    .map(|id_str| sniffer_core::ui::widget::fnv1a(id_str.as_bytes()));
            }
            app.kinetic_scrolls.clear();
        }

        if let Some((clicked_btn, _)) = find_clicked_button_with_scroll(
            root_element,
            layout_tree,
            clicked_pt,
            &|id_opt, sx, sy| resolve_active_scroll(&app.scroll_physics, id_opt, sx, sy),
        ) {
            app.event_bus.push(UiEvent::PointerDown(clicked_btn));
        } else {
            app.event_bus.push(UiEvent::PointerDown(0));
        }
    }

    for &(dx, dy) in &input.drag_events {
        let s_dx = dx * scale_x;
        let s_dy = dy * scale_y;
        app.total_drag_dist += s_dx.abs() + s_dy.abs();
        app.last_drag_delta = (s_dx, s_dy);

        app.drag_history
            .push_back((s_dx, s_dy, std::time::Instant::now()));
        if app.drag_history.len() > 8 {
            app.drag_history.pop_front();
        }

        if app.active_scrollview_drag.is_none() {
            if let Some((
                Element::ScrollView {
                    id, capture_drag, ..
                },
                _,
            )) = sniffer_render::draw::find_hovered_scrollview(
                root_element,
                layout_tree,
                scaled_last_mouse_pos,
            ) {
                if capture_drag.unwrap_or(true) {
                    app.active_scrollview_drag = id
                        .as_deref()
                        .map(|id_str| sniffer_core::ui::widget::fnv1a(id_str.as_bytes()));
                }
            }
        }

        if let Some(sv_id) = app.active_scrollview_drag {
            if let Some(phys) = app.scroll_physics.get_mut(&sv_id) {
                phys.apply_drag(s_dx);
                phys.is_dragging = true;
                phys.snap_target_x = None;
            } else {
                let target = sniffer_render::draw::find_hovered_scrollview(
                    root_element,
                    layout_tree,
                    scaled_last_mouse_pos,
                );
                let max_scroll = target.map_or((0.0, 0.0), |(_, lay)| get_max_scroll_for_lay(lay));
                app.event_bus.push(UiEvent::Scroll(
                    Some(sv_id),
                    s_dx,
                    s_dy,
                    max_scroll.0,
                    max_scroll.1,
                ));
            }
        }
    }

    if input.mouse_released {
        if let Some(sv_id) = app.active_scrollview_drag {
            if app.scroll_physics.contains_key(&sv_id) {
                let now = std::time::Instant::now();
                let cutoff = now
                    .checked_sub(std::time::Duration::from_millis(150))
                    .unwrap_or(now);
                let mut recent_count = 0;
                let mut total_dx = 0.0;
                let mut first_time = None;
                let mut last_time = None;

                for &(dx, _, t) in &app.drag_history {
                    if t >= cutoff {
                        if first_time.is_none() {
                            first_time = Some(t);
                        }
                        last_time = Some(t);
                        total_dx += dx;
                        recent_count += 1;
                    }
                }

                let vel_x = if recent_count >= 2 {
                    let dt_recent = last_time
                        .unwrap()
                        .duration_since(first_time.unwrap())
                        .as_secs_f32()
                        .max(0.001);
                    total_dx / dt_recent
                } else if let Some(last) = last_time {
                    let dt_recent = now.duration_since(last).as_secs_f32().max(0.001);
                    app.last_drag_delta.0 / dt_recent
                } else {
                    0.0
                };

                if let Some(phys) = app.scroll_physics.get_mut(&sv_id) {
                    phys.release_drag(vel_x);
                }
            } else {
                let momentum_enabled = if let Some((
                    Element::ScrollView {
                        momentum_scrolling, ..
                    },
                    _,
                )) = sniffer_render::draw::find_hovered_scrollview(
                    root_element,
                    layout_tree,
                    scaled_last_mouse_pos,
                ) {
                    momentum_scrolling.unwrap_or(true)
                } else {
                    true
                };

                if momentum_enabled {
                    app.kinetic_scrolls.push(KineticScroll {
                        sv_id,
                        velocity_x: app.last_drag_delta.0,
                        velocity_y: app.last_drag_delta.1,
                    });
                }
            }
        }

        let released_btn = find_hovered_button_with_scroll(
            root_element,
            layout_tree,
            scaled_last_mouse_pos,
            &|id_opt, sx, sy| resolve_active_scroll(&app.scroll_physics, id_opt, sx, sy),
        )
        .map(|(id, _)| id);
        app.event_bus.push(UiEvent::PointerUp(released_btn));

        if app.total_drag_dist < 15.0 {
            if let Some(down_pt) = app.mouse_down_pos {
                if let Some((clicked_btn, rect)) = find_clicked_button_with_scroll(
                    root_element,
                    layout_tree,
                    down_pt,
                    &|id_opt, sx, sy| resolve_active_scroll(&app.scroll_physics, id_opt, sx, sy),
                )
                .or_else(|| {
                    find_clicked_button_with_scroll(
                        root_element,
                        layout_tree,
                        scaled_last_mouse_pos,
                        &|id_opt, sx, sy| {
                            resolve_active_scroll(&app.scroll_physics, id_opt, sx, sy)
                        },
                    )
                }) {
                    app.event_bus
                        .push(UiEvent::Click(clicked_btn, rect.width, rect.height));
                } else {
                    app.event_bus.push(UiEvent::ClickOutside);
                }
            }
        }

        app.active_scrollview_drag = None;
        app.last_drag_delta = (0.0, 0.0);
        app.drag_history.clear();
        app.mouse_down_pos = None;
    }

    for &(x, y) in &input.scroll_events {
        let target = sniffer_render::draw::find_hovered_scrollview(
            root_element,
            layout_tree,
            scaled_last_mouse_pos,
        )
        .or_else(|| find_first_scrollview(root_element, layout_tree));

        if let Some((
            Element::ScrollView {
                id,
                scroll_sensitivity,
                dynamic_sensitivity,
                ..
            },
            lay,
        )) = target
        {
            let mut factor = scroll_sensitivity.unwrap_or(1.0);
            if dynamic_sensitivity.unwrap_or(false) && !lay.children.is_empty() {
                let view_height = lay.rect.height;
                let mut min_y = f32::MAX;
                let mut max_y = f32::MIN;
                for child in &lay.children {
                    if child.rect.y < min_y {
                        min_y = child.rect.y;
                    }
                    if child.rect.y + child.rect.height > max_y {
                        max_y = child.rect.y + child.rect.height;
                    }
                }
                let content_height = max_y - min_y;
                if view_height > 0.0 && content_height > view_height {
                    factor *= content_height / view_height;
                }
            }

            let sv_id = id
                .as_deref()
                .map(|id_str| sniffer_core::ui::widget::fnv1a(id_str.as_bytes()));

            if let Some(wid) = sv_id {
                if let Some(phys) = app.scroll_physics.get_mut(&wid) {
                    let delta = -x * 30.0 - y * 30.0;
                    phys.apply_drag(delta);
                    phys.release_drag(delta * 4.0);
                } else {
                    let max_scroll = get_max_scroll_for_lay(lay);
                    app.event_bus.push(UiEvent::Scroll(
                        Some(wid),
                        -x * 20.0 * factor,
                        -y * 20.0 * factor,
                        max_scroll.0,
                        max_scroll.1,
                    ));
                }
            }
        }
    }

    app.kinetic_scrolls.retain_mut(|k| {
        if k.velocity_x.abs() > 0.1 || k.velocity_y.abs() > 0.1 {
            let max_scroll = find_first_scrollview(root_element, layout_tree)
                .map_or((0.0, 0.0), |(_, lay)| get_max_scroll_for_lay(lay));
            app.event_bus.push(UiEvent::Scroll(
                Some(k.sv_id),
                k.velocity_x,
                k.velocity_y,
                max_scroll.0,
                max_scroll.1,
            ));
            k.velocity_x *= 0.92;
            k.velocity_y *= 0.92;
            true
        } else {
            false
        }
    });
}
