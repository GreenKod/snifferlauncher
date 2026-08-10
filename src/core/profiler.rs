//! Zero-cost Frame Time Profiler & Performance Monitor
//!
//! Active under `#[cfg(feature = "devkit")]`.

use std::collections::VecDeque;
use std::time::Instant;

pub struct FrameProfiler {
    frame_times: VecDeque<f32>,
    cpu_times: VecDeque<f32>,
    gpu_times: VecDeque<f32>,
    swap_times: VecDeque<f32>,
    max_history: usize,
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

pub fn count_elements(element: &crate::core::types::Element) -> usize {
    match element {
        crate::core::types::Element::Container { children, .. }
        | crate::core::types::Element::ScrollView { children, .. }
        | crate::core::types::Element::SharedView { children, .. } => {
            1 + children.iter().map(count_elements).sum::<usize>()
        }
        _ => 1,
    }
}

pub fn render_devkit_hud(
    renderer: &mut dyn crate::core::render::api::Renderer,
    profiler: &FrameProfiler,
    node_count: usize,
    screen_width: f32,
) {
    let fps = profiler.current_fps();
    let avg_ms = profiler.average_frame_time_ms();
    let cpu_ms = profiler.avg_cpu_ms();
    let gpu_ms = profiler.avg_gpu_ms();
    let swap_ms = profiler.avg_swap_ms();

    let w = 210.0_f32;
    let h = 100.0_f32;
    let x = (screen_width - w - 16.0).max(10.0);
    let y = 50.0_f32;

    // Outer Card
    renderer.draw_rect(
        crate::core::Rect::new(x, y, w, h),
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
    renderer.draw_text(&fps_str, x + 12.0, y + 28.0, 13.0, fps_color);

    // Latency details
    let cpu_gpu_str = format!("CPU: {cpu_ms:.1}ms | GPU: {gpu_ms:.1}ms");
    renderer.draw_text(&cpu_gpu_str, x + 12.0, y + 50.0, 11.0, 0xFFE2_E8F0);

    let os_str = format!("VSync: {swap_ms:.1}ms | Nodes: {node_count}");
    renderer.draw_text(&os_str, x + 12.0, y + 68.0, 11.0, 0xFF94_A3B8);
}

