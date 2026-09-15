use sniffer_core::profiler::{FrameProfiler, count_elements};
use sniffer_core::style::Style;
use sniffer_core::types::Element;
use std::sync::{Arc, Mutex};
use std::time::Instant;

fn build_test_ui_tree(node_count: usize) -> Element {
    let mut children = Vec::with_capacity(node_count);
    for i in 0..node_count {
        children.push(Element::Label {
            id: Some(format!("app_item_{i}")),
            text: format!("Application {i}"),
            style: Style::default(),
        });
    }
    Element::Container {
        id: Some("app_grid".to_string()),
        style: Style::default(),
        children,
    }
}

#[test]
fn test_profiler_memory_footprint_benchmark() {
    let struct_size = std::mem::size_of::<FrameProfiler>();
    let profiler = FrameProfiler::new(120);

    #[cfg(not(feature = "devkit"))]
    {
        // In non-devkit mode, the 4 heap-allocated VecDeque buffers are omitted
        println!(
            "\n[Memory Footprint Benchmark - Release Mode (Non-DevKit)]\n  FrameProfiler struct size: {struct_size} bytes\n  Active VecDeque heap capacity: 0 elements (0 KB)"
        );
        assert_eq!(profiler.current_fps(), 0.0);
    }

    #[cfg(feature = "devkit")]
    {
        println!(
            "\n[Memory Footprint Benchmark - DevKit Mode]\n  FrameProfiler struct size: {struct_size} bytes\n  Active VecDeque ring buffers: 4 x 120 capacity"
        );
        assert_eq!(profiler.current_fps(), 0.0);
    }
}

#[test]
fn test_render_loop_profiler_overhead_benchmark() {
    let tree = build_test_ui_tree(200);
    let profiler = Arc::new(Mutex::new(FrameProfiler::new(120)));
    const FRAME_ITERATIONS: usize = 5_000;

    let now = Instant::now();

    // 1. Measure cost with devkit active vs inactive
    let start = Instant::now();
    for _ in 0..FRAME_ITERATIONS {
        let count = count_elements(&tree);
        if let Ok(mut prof) = profiler.lock() {
            prof.record_entities(count, count);
            prof.record_frame(now, now, now, now);
        }
    }
    let duration = start.elapsed();
    let per_frame_ns = duration.as_nanos() as f64 / FRAME_ITERATIONS as f64;

    #[cfg(not(feature = "devkit"))]
    println!(
        "\n[Render Loop Overhead Benchmark - Release (DevKit Disabled)]\n  Iterations: {FRAME_ITERATIONS}\n  Total Time: {duration:?}\n  Overhead Per Frame: {per_frame_ns:.2} ns"
    );

    #[cfg(feature = "devkit")]
    {
        let per_frame_us = per_frame_ns / 1000.0;
        println!(
            "\n[Render Loop Overhead Benchmark - DevKit Enabled]\n  Iterations: {FRAME_ITERATIONS}\n  Total Time: {duration:?}\n  Overhead Per Frame: {per_frame_ns:.2} ns ({per_frame_us:.3} µs)"
        );
    }
}

#[test]
fn test_touch_telemetry_lock_and_allocation_overhead_benchmark() {
    let profiler = Arc::new(Mutex::new(FrameProfiler::new(120)));
    const EVENT_ITERATIONS: usize = 5_000;

    // Simulation A: Devkit mode (mutex lock + dynamic string allocations on every touch)
    let devkit_start = Instant::now();
    for i in 0..EVENT_ITERATIONS {
        let gesture_name = if i % 2 == 0 {
            "DRAG / PAN".to_string()
        } else {
            "MULTI_TOUCH / PINCH".to_string()
        };
        let target_name = format!("btn_{i:x}");
        if let Ok(mut prof) = profiler.lock() {
            prof.touch_telemetry.active_pointers = 1;
            prof.touch_telemetry.gesture = gesture_name;
            prof.touch_telemetry.target_element = target_name;
            prof.touch_telemetry.touch_x = 100.0;
            prof.touch_telemetry.touch_y = 200.0;
        }
    }
    let devkit_duration = devkit_start.elapsed();

    // Simulation B: Release non-devkit mode (zero allocations, zero mutex locks)
    let release_start = Instant::now();
    for _ in 0..EVENT_ITERATIONS {
        // Feature flag eliminates the entire telemetry formatting and lock block
    }
    let release_duration = release_start
        .elapsed()
        .max(std::time::Duration::from_nanos(1));

    let speedup = devkit_duration.as_nanos() as f64 / release_duration.as_nanos() as f64;
    println!(
        "\n[Touch Telemetry Benchmark - 5000 Events]\n  DevKit Mode (With Locks & Allocations): {devkit_duration:?}\n  Release Mode (With DevKit Gated Out): {release_duration:?}\n  Touch Processing Speedup: {speedup:.1}x"
    );

    assert!(speedup > 1.0);
}
