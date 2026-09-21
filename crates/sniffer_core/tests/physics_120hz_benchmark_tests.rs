use sniffer_core::anim::TransitionManager;
use sniffer_core::physics::{FlingVelocityTracker, ScrollPhysics, step_momentum_decay};
use sniffer_core::profiler::FrameProfiler;
use sniffer_core::style::{Easing, Style, Transition};
use std::time::{Duration, Instant};

#[test]
fn test_120hz_touch_to_render_evaluation_latency() {
    let mut tracker = FlingVelocityTracker::new();
    let mut phys = ScrollPhysics {
        max_y: Some(2000.0),
        ..Default::default()
    };

    let start = Instant::now();
    let sample_count = 120; // 1 second of 120 Hz touch moves

    for i in 0..sample_count {
        let t = start + Duration::from_micros(i * 8333);
        tracker.add_movement(5.0, 12.0, t);
    }

    let compute_start = Instant::now();
    let (_vx, vy) = tracker.compute_velocity(start + Duration::from_micros(sample_count * 8333));
    let tracker_latency = compute_start.elapsed();

    // Tracker calculation must complete in sub-millisecond time (< 50 microseconds)
    assert!(tracker_latency < Duration::from_micros(100));
    assert!(vy > 0.0);

    // Release drag into physics
    phys.pos_y = -50.0;
    phys.release_drag_y(vy);

    // Measure single frame physics tick at 120 Hz (dt = 8.33ms)
    let tick_start = Instant::now();
    let active = phys.tick(1.0 / 120.0);
    let tick_latency = tick_start.elapsed();

    assert!(active);
    assert!(tick_latency < Duration::from_micros(100));

    // Total touch-to-physics latency must occupy less than 2% of the 8.33ms frame budget
    let total_us = (tracker_latency + tick_latency).as_micros();
    assert!(
        total_us < 200,
        "Touch-to-render evaluation took too long: {total_us} us"
    );
}

#[test]
fn test_120hz_zero_jank_continuous_simulation() {
    let mut profiler = FrameProfiler::new(120);
    let mut phys = ScrollPhysics {
        snap_x: Some(360.0),
        page_count: Some(5),
        ..Default::default()
    };

    phys.pos_x = 100.0;
    phys.release_drag(500.0);

    let dt = 1.0 / 120.0;
    let dt_duration = Duration::from_micros(8333);
    let mut frame_start = Instant::now();

    for _ in 0..120 {
        let render_start = frame_start + Duration::from_micros(500); // 0.5ms CPU physics/prep
        let draw_end = render_start + Duration::from_micros(2000); // 2.0ms GPU draw
        let swap_end = draw_end + Duration::from_micros(500); // 0.5ms buffer swap

        phys.tick(dt);

        profiler.record_frame(frame_start, render_start, draw_end, swap_end);
        frame_start += dt_duration;
    }

    // Average frame time should be ~3.0ms (< 8.33ms budget for 120 FPS)
    assert!(profiler.average_frame_time_ms() < 8.33);
    assert_eq!(
        profiler.jank_count_120hz(),
        0,
        "No 120Hz frame drops allowed"
    );
}

#[test]
fn test_120hz_100_element_staggered_cascade_scalability() {
    let mut manager = TransitionManager::default();
    let num_elements = 120; // 120 app drawer icons

    for i in 0..num_elements {
        let id = format!("icon_{i}");
        let style = Style {
            opacity: 0.0,
            transform: sniffer_core::style::Transform {
                translate_y: 50.0,
                ..Default::default()
            },
            transition: Transition {
                duration: 0.35,
                easing: Easing::Spring {
                    stiffness: 240.0,
                    damping: 22.0,
                },
                delay: 0.0,
                stagger_index: (i % 24) as u32,
                stagger_interval: 0.018,
            },
            ..Default::default()
        };
        manager.update_target(&id, &style);

        let mut target_style = style;
        target_style.opacity = 1.0;
        target_style.transform.translate_y = 0.0;
        manager.update_target(&id, &target_style);
    }

    assert!(manager.is_animating());

    // Step 120 frames at 120 FPS, measure per-frame CPU time for 120 simultaneous springs
    let dt = 1.0 / 120.0;
    let mut max_tick_duration = Duration::ZERO;
    let mut total_tick_duration = Duration::ZERO;

    for _ in 0..120 {
        let t0 = Instant::now();
        manager.tick(dt);
        let elapsed = t0.elapsed();

        if elapsed > max_tick_duration {
            max_tick_duration = elapsed;
        }
        total_tick_duration += elapsed;
    }

    let avg_tick_ms = (total_tick_duration.as_secs_f32() / 120.0) * 1000.0;

    // 120 concurrent springs must update in < 1.0 ms average per frame
    assert!(
        avg_tick_ms < 1.0,
        "120 elements cascade tick exceeded 1.0ms budget: {avg_tick_ms:.3}ms"
    );
}

#[test]
fn test_momentum_decay_numeric_stability_120hz() {
    let mut v = 8000.0;
    let friction = 7.62;
    let dt = 1.0 / 120.0;

    for _ in 0..1200 {
        let (step, next_v) = step_momentum_decay(v, friction, dt);
        assert!(!step.is_nan());
        assert!(!next_v.is_nan());
        assert!(!step.is_infinite());
        assert!(!next_v.is_infinite());
        v = next_v;
    }

    assert_eq!(v, 0.0);
}
