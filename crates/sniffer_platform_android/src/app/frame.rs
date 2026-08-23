use super::bridge::SharedRenderState;
use super::physics_sync;
use super::state::AppState;
use android_activity::AndroidApp;
use sniffer_core::types::Element;
use sniffer_core::ui::event::UiEvent;
use sniffer_core::{ScreenMetrics, Size, calculate_layout};
use std::sync::{Arc, RwLock};

#[allow(
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap
)]
pub fn update_and_render_state(
    app: &AndroidApp,
    state: &mut AppState,
    root_element: &mut Element,
    shared_render_state: &Arc<RwLock<Arc<SharedRenderState>>>,
    dt: f32,
    mut ui_changed: bool,
    current_ui_version: u64,
) {
    let (width, height) = if let Some(window) = app.native_window() {
        (
            f32::from(u16::try_from(window.width()).expect("window width fits in u16")),
            f32::from(u16::try_from(window.height()).expect("window height fits in u16")),
        )
    } else {
        state.cached_screen_size
    };

    if width < 1.0 || height < 1.0 {
        return;
    }

    let screen_changed = (width - state.cached_screen_size.0).abs() > 0.5
        || (height - state.cached_screen_size.1).abs() > 0.5;

    if screen_changed {
        state.cached_screen_size = (width, height);
        state.cached_safe_area =
            crate::jni::get_safe_area(app).map_or((0.0, 0.0), |(top, bottom)| {
                (
                    f32::from(i16::try_from(top).expect("safe-area top fits in i16")),
                    f32::from(i16::try_from(bottom).expect("safe-area bottom fits in i16")),
                )
            });
        state.cached_density = crate::jni::get_density();

        state.cached_layout = None;
        state.cached_max_scroll.clear();

        let content_h = height - state.cached_safe_area.0 - state.cached_safe_area.1;
        sniffer_core::types::SCREEN_WIDTH
            .store(width.to_bits(), std::sync::atomic::Ordering::Relaxed);
        sniffer_core::types::SCREEN_HEIGHT
            .store(content_h.to_bits(), std::sync::atomic::Ordering::Relaxed);

        ui_changed = true;
    }

    let is_first_frame =
        matches!(root_element, Element::Container { children, .. } if children.is_empty());
    if ui_changed || is_first_frame {
        if let Some(new_root) = state.plugin_registry.build_ui() {
            *root_element = new_root;
            state.layout_dirty = true;
        }
    }

    if screen_changed || is_first_frame {
        let content_h = height - state.cached_safe_area.0 - state.cached_safe_area.1;
        state
            .virtual_page_manager
            .on_viewport_resized(width, content_h, root_element);
    }

    state.virtual_page_manager.virtualize_tree(root_element);
    physics_sync::sync_scroll_physics_from_tree(root_element, &mut state.scroll_physics);
    state.transition_manager.sync_tree(root_element);
    let _ = state.transition_manager.tick(dt);

    let (safe_area_top, safe_area_bottom) = state.cached_safe_area;
    let (density, scaled_density) = state.cached_density;

    let metrics = ScreenMetrics::from_scale(width, height, density, scaled_density);

    if let Some(ref cached) = state.cached_layout {
        let cached_is_landscape = cached.rect.width > cached.rect.height;
        let window_is_landscape = width > height;
        if cached_is_landscape != window_is_landscape {
            state.cached_layout = None;
            state.layout_dirty = true;
            state
                .virtual_page_manager
                .invalidate_on_resize(root_element);
        }
    }

    let layout_tree = if !state.layout_dirty
        && let Some(ref cached) = state.cached_layout
    {
        cached.clone()
    } else {
        state.last_ui_version = current_ui_version;
        let fresh = Arc::new(calculate_layout(
            root_element,
            Size::new(width, height - safe_area_top - safe_area_bottom),
            0.0,
            safe_area_top,
        ));
        super::state::update_max_scroll_cache(state, root_element, &fresh);
        state.cached_layout = Some(fresh.clone());
        state.layout_dirty = false;
        fresh
    };

    let get_max_scroll = |target_id: Option<u64>| -> f32 {
        target_id
            .and_then(|id| state.cached_max_scroll.get(&id).copied())
            .unwrap_or(0.0)
    };

    state.kinetic_scrolls.retain_mut(|k| {
        if k.velocity_x.abs() > 0.5 || k.velocity_y.abs() > 0.5 {
            state.event_bus.push(UiEvent::Scroll(
                Some(k.sv_id),
                k.velocity_x,
                k.velocity_y,
                999_999.0,
                get_max_scroll(Some(k.sv_id)),
            ));
            k.velocity_x *= 0.88;
            k.velocity_y *= 0.88;
            true
        } else {
            false
        }
    });

    let content_h_for_js = height - safe_area_top - safe_area_bottom;
    let vmin_px = width.min(content_h_for_js) / 100.0;
    let phys_ids: Vec<u64> = state.scroll_physics.keys().copied().collect();
    for sv_id in phys_ids {
        if let Some(phys) = state.scroll_physics.get_mut(&sv_id) {
            let _ = phys.tick(dt);

            if let Some(snap_width) = phys.snap_x {
                if snap_width > 0.0 {
                    let max_page = (phys.page_count.unwrap_or(1) as i32 - 1).max(0);
                    let current_page = (phys.pos_x / snap_width).round() as i32;
                    let clamped_page = current_page.clamp(0, max_page) as usize;

                    let page_window_changed = state.virtual_page_manager.update_predicted_page(
                        clamped_page,
                        root_element,
                        phys.vel_x,
                    );

                    if page_window_changed {
                        state.cached_layout = None;
                    }

                    phys.last_snap_page = clamped_page as i32;

                    if phys.snap_just_completed && phys.on_snap.is_some() {
                        state.event_bus.push(UiEvent::PageSnapped {
                            widget_id: sv_id,
                            page: clamped_page as i32,
                        });
                    }

                    let indicator_changed = physics_sync::update_indicator_dots_in_element(
                        root_element,
                        phys.last_snap_page,
                        vmin_px,
                    );

                    if indicator_changed {
                        state.cached_layout = None;
                        state.layout_dirty = true;
                    }
                }
            }

            let pos_x = phys.pos_x;
            let pos_y = phys.pos_y;
            physics_sync::inject_physics_to_tree(root_element, sv_id, pos_x, pos_y);
        }
    }

    state.plugin_registry.tick();

    state.plugin_registry.dispatch(
        &state.event_bus,
        &state.style_map,
        &state.data_map,
        &state.action_queue,
    );

    super::actions::process_android_actions(&state.action_queue, app);

    let current_warmed_up = shared_render_state.read().unwrap().shaders_warmed_up;
    *shared_render_state.write().unwrap() = Arc::new(SharedRenderState {
        root_element: root_element.clone(),
        layout_tree,
        metrics,
        style_map: state.style_map.clone(),
        data_map: state.data_map.clone(),
        transition_manager: state.transition_manager.clone(),
        shaders_warmed_up: current_warmed_up,
    });
}
