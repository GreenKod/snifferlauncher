use sniffer_core::profiler::{FrameProfiler, count_elements};
#[cfg(feature = "devkit")]
use std::time::Duration;
use std::time::Instant;

#[cfg(feature = "devkit")]
#[test]
fn test_frame_profiler_record_and_fps() {
    let mut profiler = FrameProfiler::new(60);

    let base = Instant::now();
    for i in 0..60 {
        let t0 = base + Duration::from_millis(i * 11);
        let t1 = t0 + Duration::from_millis(1);
        let t2 = t1 + Duration::from_millis(8);
        let t3 = t2 + Duration::from_millis(2);

        profiler.record_frame(t0, t1, t2, t3);
    }

    let fps = profiler.current_fps();
    assert!(fps > 0.0, "FPS was: {fps}");

    let avg_ms = profiler.average_frame_time_ms();
    assert!(avg_ms > 0.0, "Avg frame time was: {avg_ms}");

    let cpu = profiler.avg_cpu_ms();
    assert!(cpu > 0.0, "Avg CPU was: {cpu}");

    let gpu = profiler.avg_gpu_ms();
    assert!(gpu > 0.0, "Avg GPU was: {gpu}");

    let child = sniffer_core::types::Element::Label {
        id: None,
        text: "child".to_string(),
        style: sniffer_core::style::Style::default(),
    };
    let container = sniffer_core::types::Element::Container {
        id: None,
        style: sniffer_core::style::Style::default(),
        children: vec![child],
    };
    assert_eq!(count_elements(&container), 2);
}

#[cfg(not(feature = "devkit"))]
#[test]
fn test_frame_profiler_no_op_zero_cost() {
    let mut profiler = FrameProfiler::new(60);
    let now = Instant::now();

    profiler.record_frame(now, now, now, now);
    profiler.record_entities(10, 20);

    assert_eq!(profiler.current_fps(), 0.0);
    assert_eq!(profiler.average_frame_time_ms(), 0.0);
    assert_eq!(profiler.avg_cpu_ms(), 0.0);
    assert_eq!(profiler.avg_gpu_ms(), 0.0);
    assert_eq!(profiler.avg_swap_ms(), 0.0);

    let dummy_element = sniffer_core::types::Element::Label {
        id: None,
        text: "test".to_string(),
        style: sniffer_core::style::Style::default(),
    };
    assert_eq!(count_elements(&dummy_element), 0);
}
