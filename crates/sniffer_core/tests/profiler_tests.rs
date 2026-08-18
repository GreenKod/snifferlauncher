use sniffer_core::profiler::FrameProfiler;
use std::time::{Duration, Instant};

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
}
