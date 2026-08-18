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
        }
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

fn draw_touch_hitboxes(
    renderer: &mut dyn crate::render_api::Renderer,
    element: &crate::types::Element,
    layout: &crate::layout::LayoutNode,
) {
    let elem_id = match element {
        crate::types::Element::Container { id, .. }
        | crate::types::Element::ScrollView { id, .. }
        | crate::types::Element::SharedView { id, .. }
        | crate::types::Element::Label { id, .. }
        | crate::types::Element::Image { id, .. }
        | crate::types::Element::TextInput { id, .. }
        | crate::types::Element::Slider { id, .. }
        | crate::types::Element::ProgressBar { id, .. }
        | crate::types::Element::Checkbox { id, .. } => id,
    };

    let is_button = match elem_id {
        Some(k) => {
            k.starts_with("btn_")
                || k.starts_with("dock_app_btn_")
                || k.starts_with("dock_app_circle_")
                || k.starts_with("fab_")
                || k.starts_with("app_card_")
                || k.starts_with("card_")
                || k.starts_with("action_")
                || k.starts_with("clock_")
                || k.contains("btn")
                || k.contains("card")
                || k.contains("item")
        }
        None => matches!(
            element,
            crate::types::Element::TextInput { .. }
                | crate::types::Element::Checkbox { .. }
        ),
    };

    if is_button && layout.rect.width > 2.0 && layout.rect.height > 2.0 {
        // Dock buttons have green/emerald accent, other buttons have cyan/sky accent
        let is_dock = elem_id.as_ref().is_some_and(|k| k.contains("dock"));
        let (fill_color, border_color) = if is_dock {
            (0x284A_DE80_u32, Some(0xEE4A_DE80_u32))
        } else {
            (0x2038_BDF8_u32, Some(0xDD38_BDF8_u32))
        };

        // Draw bounding box
        renderer.draw_rect(
            layout.rect,
            fill_color,
            4.0,
            1.5,
            border_color,
        );
    }

    match element {
        crate::types::Element::ScrollView {
            children,
            scroll_x,
            scroll_y,
            ..
        } => {
            let safe_scroll_y = if scroll_y.is_nan() || scroll_y.is_infinite() { 0.0 } else { *scroll_y };
            let safe_scroll_x = if scroll_x.is_nan() || scroll_x.is_infinite() { 0.0 } else { *scroll_x };
            renderer.push_clip_rect(layout.rect, 8.0);
            renderer.push_transform(0.0, 0.0, 1.0, 0.0, -safe_scroll_x, -safe_scroll_y);
            for (child_el, child_lay) in children.iter().zip(layout.children.iter()) {
                draw_touch_hitboxes(renderer, child_el, child_lay);
            }
            renderer.pop_transform();
            renderer.pop_clip_rect();
        }
        crate::types::Element::Container { children, .. }
        | crate::types::Element::SharedView { children, .. } => {
            for (child_el, child_lay) in children.iter().zip(layout.children.iter()) {
                draw_touch_hitboxes(renderer, child_el, child_lay);
            }
        }
        _ => {}
    }
}

pub fn render_devkit_hud(
    renderer: &mut dyn crate::render_api::Renderer,
    profiler: &FrameProfiler,
    node_count: usize,
    _screen_width: f32,
    root_element: &crate::types::Element,
    layout_tree: &crate::layout::LayoutNode,
) {
    // 1. Dokunma ve Hitbox Alanlarını Çiz
    draw_touch_hitboxes(renderer, root_element, layout_tree);

    // 2. Aktif Dokunma Noktası (Pointer Telemetry Visualizer)
    if profiler.touch_telemetry.touch_x > 0.0 || profiler.touch_telemetry.touch_y > 0.0 {
        let tx = profiler.touch_telemetry.touch_x;
        let ty = profiler.touch_telemetry.touch_y;
        renderer.draw_circle(tx, ty, 20.0, 0x44FB_BF24);
        renderer.draw_circle(tx, ty, 10.0, 0x88FB_BF24);
        renderer.draw_circle(tx, ty, 4.0, 0xFFF5_9E0B);
    }

    // 3. Devkit HUD Bilgi Kartı
    let fps = profiler.current_fps();
    let avg_ms = profiler.average_frame_time_ms();
    let cpu_ms = profiler.avg_cpu_ms();
    let gpu_ms = profiler.avg_gpu_ms();
    let swap_ms = profiler.avg_swap_ms();

    let w = 240.0_f32;
    let h = 135.0_f32;
    let x = 16.0_f32;
    let y = 16.0_f32;

    // Outer Card
    renderer.draw_rect(
        crate::Rect::new(x, y, w, h),
        0xDD0F_172A,
        10.0,
        1.5,
        Some(0xFF38_BDF8),
    );

    // Title
    renderer.draw_text("⚡ DEVKIT MONITOR", x + 12.0, y + 10.0, 11.0, 0xFF38_BDF8);

    // FPS badge
    let fps_color = if fps >= 55.0 {
        0xFF4A_DE80
    } else if fps >= 30.0 {
        0xFFFB_BF24
    } else {
        0xFFF8_7171
    };

    let fps_str = format!("FPS: {fps:.1} ({avg_ms:.1}ms)");
    renderer.draw_text(&fps_str, x + 12.0, y + 26.0, 12.0, fps_color);

    // Latency details
    let cpu_gpu_str = format!("CPU: {cpu_ms:.1}ms | GPU: {gpu_ms:.1}ms");
    renderer.draw_text(&cpu_gpu_str, x + 12.0, y + 44.0, 10.0, 0xFFE2_E8F0);

    let os_str = format!("VSync: {swap_ms:.1}ms | Nodes: {node_count}");
    renderer.draw_text(&os_str, x + 12.0, y + 58.0, 10.0, 0xFF94_A3B8);

    // Touch Telemetry
    let touch = &profiler.touch_telemetry;
    let gesture_name = if touch.gesture.is_empty() { "IDLE" } else { &touch.gesture };
    let touch_hdr = format!("👉 Pointers: {} | {}", touch.active_pointers, gesture_name);
    renderer.draw_text(&touch_hdr, x + 12.0, y + 76.0, 10.0, 0xFFFACC15);

    let target_name = if touch.target_element.is_empty() { "None" } else { &touch.target_element };
    let target_str = format!("Target: {target_name}");
    renderer.draw_text(&target_str, x + 12.0, y + 92.0, 10.0, 0xFFE2_E8F0);

    let pos_str = format!("Pos: ({:.1}, {:.1})", touch.touch_x, touch.touch_y);
    renderer.draw_text(&pos_str, x + 12.0, y + 108.0, 10.0, 0xFF94_A3B8);
}

