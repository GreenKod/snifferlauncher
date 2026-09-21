use sniffer_core::physics::{FlingConfig, FlingVelocityTracker, step_momentum_decay};
use std::time::{Duration, Instant};

#[test]
fn test_custom_fling_config() {
    let config = FlingConfig {
        window_duration: Duration::from_millis(200),
        stationary_threshold: Duration::from_millis(100),
        decay_half_life: Duration::from_millis(50),
        min_velocity: 10.0,
        max_velocity: 5000.0,
    };
    let tracker = FlingVelocityTracker::with_config(config);
    assert_eq!(tracker.config.max_velocity, 5000.0);
    assert_eq!(tracker.config.min_velocity, 10.0);
}

#[test]
fn test_fling_velocity_tracker_basic() {
    let mut tracker = FlingVelocityTracker::new();
    let t0 = Instant::now();

    // Constant movement: 100px every 16ms -> ~6250 px/s
    tracker.add_movement(100.0, -50.0, t0);
    tracker.add_movement(100.0, -50.0, t0 + Duration::from_millis(16));
    tracker.add_movement(100.0, -50.0, t0 + Duration::from_millis(32));

    let (vx, vy) = tracker.compute_velocity(t0 + Duration::from_millis(33));
    assert!((5000.0..=8000.0).contains(&vx));
    assert!((-8000.0..=-2500.0).contains(&vy));
}

#[test]
fn test_fling_stationary_pause_cutoff() {
    let mut tracker = FlingVelocityTracker::new();
    let t0 = Instant::now();

    tracker.add_movement(100.0, 100.0, t0);
    // Pause for 70ms before release (exceeds default stationary threshold 60ms)
    let (vx, vy) = tracker.compute_velocity(t0 + Duration::from_millis(70));
    assert_eq!(vx, 0.0);
    assert_eq!(vy, 0.0);
}

#[test]
fn test_fling_sub_threshold_velocity_deadband() {
    let mut tracker = FlingVelocityTracker::new();
    let t0 = Instant::now();

    // Extremely slow crawl: 0.2px in 50ms -> 4 px/s (< 50 px/s min threshold)
    tracker.add_movement(0.2, 0.1, t0);
    tracker.add_movement(0.2, 0.1, t0 + Duration::from_millis(50));

    let (vx, vy) = tracker.compute_velocity(t0 + Duration::from_millis(52));
    assert_eq!(vx, 0.0);
    assert_eq!(vy, 0.0);
}

#[test]
fn test_momentum_decay_step_and_convergence() {
    let v0 = 5000.0;
    let friction = 7.62;
    let dt = 1.0 / 120.0;

    let mut current_v = v0;
    let mut total_displacement = 0.0;
    let mut steps = 0;

    while current_v > 0.5 && steps < 600 {
        let (dx, next_v) = step_momentum_decay(current_v, friction, dt);
        total_displacement += dx;
        current_v = next_v;
        steps += 1;
    }

    // Theoretical total displacement: v0 / lambda = 5000 / 7.62 ≈ 656.17
    let theoretical = v0 / friction;
    assert!((total_displacement - theoretical).abs() < 1.0);
    assert!(current_v <= 0.5);
}
