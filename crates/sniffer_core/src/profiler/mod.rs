//! Zero-cost Frame Time Profiler & Performance Monitor
//!
//! Active under `#[cfg(feature = "devkit")]`.

use std::collections::VecDeque;
use std::time::Instant;

#[derive(Clone, Debug, Default)]
pub struct TouchTelemetry {
    pub active_pointers: usize,
    pub gesture: String,
    pub target_element: String,
    pub touch_x: f32,
    pub touch_y: f32,
}

pub struct FrameProfiler {
    frame_times: VecDeque<f32>,
    cpu_times: VecDeque<f32>,
    gpu_times: VecDeque<f32>,
    swap_times: VecDeque<f32>,
    max_history: usize,
    pub touch_telemetry: TouchTelemetry,
    pub rendered_entities: usize,
    pub total_entities: usize,
}

impl FrameProfiler {
    #[must_use]
    pub fn new(max_history: usize) -> Self {
        Self {
            frame_times: VecDeque::with_capacity(max_history),
            cpu_times: VecDeque::with_capacity(max_history),
            gpu_times: VecDeque::with_capacity(max_history),
            swap_times: VecDeque::with_capacity(max_history),
            max_history,
            touch_telemetry: TouchTelemetry::default(),
            rendered_entities: 0,
            total_entities: 0,
        }
    }

    pub fn record_entities(&mut self, rendered: usize, total: usize) {
        self.rendered_entities = rendered;
        self.total_entities = total;
    }

    pub fn record_frame(
        &mut self,
        frame_start: Instant,
        render_start: Instant,
        draw_end: Instant,
        swap_end: Instant,
    ) {
        let cpu_ms = render_start.duration_since(frame_start).as_secs_f32() * 1000.0;
        let gpu_ms = draw_end.duration_since(render_start).as_secs_f32() * 1000.0;
        let swap_ms = swap_end.duration_since(draw_end).as_secs_f32() * 1000.0;
        let total_ms = swap_end.duration_since(frame_start).as_secs_f32() * 1000.0;

        let push_limit = |vec: &mut VecDeque<f32>, val: f32, limit: usize| {
            if vec.len() >= limit {
                vec.pop_front();
            }
            vec.push_back(val);
        };

        push_limit(&mut self.frame_times, total_ms, self.max_history);
        push_limit(&mut self.cpu_times, cpu_ms, self.max_history);
        push_limit(&mut self.gpu_times, gpu_ms, self.max_history);
        push_limit(&mut self.swap_times, swap_ms, self.max_history);
    }

    #[allow(clippy::cast_precision_loss)]
    fn avg(vec: &VecDeque<f32>) -> f32 {
        if vec.is_empty() {
            return 0.0;
        }
        let sum: f32 = vec.iter().sum();
        sum / (vec.len() as f32)
    }

    #[must_use]
    pub fn average_frame_time_ms(&self) -> f32 {
        Self::avg(&self.frame_times)
    }

    #[must_use]
    pub fn avg_cpu_ms(&self) -> f32 {
        Self::avg(&self.cpu_times)
    }

    #[must_use]
    pub fn avg_gpu_ms(&self) -> f32 {
        Self::avg(&self.gpu_times)
    }

    #[must_use]
    pub fn avg_swap_ms(&self) -> f32 {
        Self::avg(&self.swap_times)
    }

    #[must_use]
    pub fn current_fps(&self) -> f32 {
        let avg = self.average_frame_time_ms();
        if avg > 0.0 {
            (1000.0 / avg).min(120.0)
        } else {
            0.0
        }
    }
}

impl Default for FrameProfiler {
    fn default() -> Self {
        Self::new(30)
    }
}

pub fn count_elements(element: &crate::types::Element) -> usize {
    match element {
        crate::types::Element::Container { children, .. }
        | crate::types::Element::ScrollView { children, .. }
        | crate::types::Element::SharedView { children, .. } => {
            let mut count = 1;
            for child in children {
                count += count_elements(child);
            }
            count
        }
        _ => 1,
    }
}

#[cfg(feature = "devkit")]
fn draw_touch_hitboxes(
    renderer: &mut dyn crate::render::Renderer,
    element: &crate::types::Element,
    layout: &crate::layout::LayoutNode,
    parent_opacity: f32,
) {
    let effective_opacity = parent_opacity * element.style().opacity;
    if effective_opacity < 0.05 {
        return;
    }

    let elem_id = element.id();
    let is_button = match elem_id {
        Some(k) => {
            !crate::ui::widget::is_structural_layout_id(k)
                && (k.starts_with("app_item_")
                    || k.starts_with("dock_app_btn_")
                    || k.starts_with("dock_app_circle_")
                    || k.starts_with("fab_")
                    || k.starts_with("app_card_")
                    || k.starts_with("btn_")
                    || k.starts_with("action_")
                    || k.starts_with("clock_")
                    || k.contains("btn")
                    || k.contains("card")
                    || k.contains("item"))
        }
        None => matches!(
            element,
            crate::types::Element::TextInput { .. } | crate::types::Element::Checkbox { .. }
        ),
    };

    if is_button && layout.rect.width > 2.0 && layout.rect.height > 2.0 {
        let is_dock = elem_id.as_ref().is_some_and(|k| k.contains("dock"));
        let (fill_color, border_color) = if is_dock {
            (0x284A_DE80_u32, Some(0xEE4A_DE80_u32))
        } else {
            (0x2038_BDF8_u32, Some(0xDD38_BDF8_u32))
        };

        renderer.draw_rect(layout.rect, fill_color, 4.0, 1.5, border_color);
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
    let has_tx = safe_tx.abs() > 0.001 || safe_ty.abs() > 0.001;

    match element {
        crate::types::Element::ScrollView {
            children,
            scroll_x,
            scroll_y,
            ..
        } => {
            let safe_scroll_y = if scroll_y.is_nan() || scroll_y.is_infinite() {
                0.0
            } else {
                *scroll_y
            };
            let safe_scroll_x = if scroll_x.is_nan() || scroll_x.is_infinite() {
                0.0
            } else {
                *scroll_x
            };
            renderer.push_clip_rect(layout.rect, 8.0);
            renderer.push_transform(
                0.0,
                0.0,
                1.0,
                0.0,
                safe_tx - safe_scroll_x,
                safe_ty - safe_scroll_y,
            );
            for (child_el, child_lay) in children.iter().zip(layout.children.iter()) {
                draw_touch_hitboxes(renderer, child_el, child_lay, effective_opacity);
            }
            renderer.pop_transform();
            renderer.pop_clip_rect();
        }
        crate::types::Element::Container { children, .. }
        | crate::types::Element::SharedView { children, .. } => {
            if has_tx {
                renderer.push_transform(0.0, 0.0, 1.0, 0.0, safe_tx, safe_ty);
            }
            for (child_el, child_lay) in children.iter().zip(layout.children.iter()) {
                draw_touch_hitboxes(renderer, child_el, child_lay, effective_opacity);
            }
            if has_tx {
                renderer.pop_transform();
            }
        }
        _ => {}
    }
}

/// Renders DevKit visualizer debug overlays: touch hitboxes and active pointer circles.
#[cfg(feature = "devkit")]
pub fn render_devkit_debug_overlays(
    renderer: &mut dyn crate::render::Renderer,
    profiler: &FrameProfiler,
    root_element: &crate::types::Element,
    layout_tree: &crate::layout::LayoutNode,
) {
    static SHOW_DEBUG_OVERLAYS: std::sync::atomic::AtomicBool =
        std::sync::atomic::AtomicBool::new(false);
    static CHECKED_ENV: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

    if !CHECKED_ENV.load(std::sync::atomic::Ordering::Relaxed) {
        let enabled = std::env::var("SNIFFER_DEBUG_HITBOXES").is_ok()
            || std::env::var("SNIFFER_DEBUG_OVERLAYS").is_ok();
        SHOW_DEBUG_OVERLAYS.store(enabled, std::sync::atomic::Ordering::Relaxed);
        CHECKED_ENV.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    if !SHOW_DEBUG_OVERLAYS.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }

    // 1. Draw touch & hitbox bounding areas
    draw_touch_hitboxes(renderer, root_element, layout_tree, 1.0);

    // 2. Active Pointer / Touch Telemetry Visualizer
    if profiler.touch_telemetry.touch_x > 0.0 || profiler.touch_telemetry.touch_y > 0.0 {
        let tx = profiler.touch_telemetry.touch_x;
        let ty = profiler.touch_telemetry.touch_y;
        renderer.draw_circle(tx, ty, 20.0, 0x44FB_BF24);
        renderer.draw_circle(tx, ty, 10.0, 0x88FB_BF24);
        renderer.draw_circle(tx, ty, 4.0, 0xFFF5_9E0B);
    }
}

/// Renders DevKit visualizer debug overlays (no-op when devkit feature is disabled).
#[cfg(not(feature = "devkit"))]
#[inline]
pub fn render_devkit_debug_overlays(
    _renderer: &mut dyn crate::render::Renderer,
    _profiler: &FrameProfiler,
    _root_element: &crate::types::Element,
    _layout_tree: &crate::layout::LayoutNode,
) {
}
