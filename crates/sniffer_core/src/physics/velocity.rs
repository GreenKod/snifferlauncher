//! High-precision touch fling velocity tracker and momentum smoothing solver.

use std::time::{Duration, Instant};

/// Configuration options for touch motion and fling velocity estimation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlingConfig {
    /// Maximum sample age retained in the velocity window (default: 150ms).
    pub window_duration: Duration,
    /// Threshold duration without motion after which touch is considered stationary (default: 60ms).
    pub stationary_threshold: Duration,
    /// Exponential decay half-life for time-weighted sample averaging (default: 40ms).
    pub decay_half_life: Duration,
    /// Minimum velocity threshold below which velocity is clamped to 0 (default: 50.0 px/s).
    pub min_velocity: f32,
    /// Maximum absolute velocity clamp (default: 8000.0 px/s).
    pub max_velocity: f32,
}

impl Default for FlingConfig {
    fn default() -> Self {
        Self {
            window_duration: Duration::from_millis(150),
            stationary_threshold: Duration::from_millis(60),
            decay_half_life: Duration::from_millis(40),
            min_velocity: 50.0,
            max_velocity: 8000.0,
        }
    }
}

/// Movement sample recorded during drag interaction.
#[derive(Debug, Clone, Copy)]
pub struct MovementSample {
    pub dx: f32,
    pub dy: f32,
    pub timestamp: Instant,
}

/// Tracks pointer motion and computes time-weighted, smooth fling velocity.
#[derive(Debug, Clone)]
pub struct FlingVelocityTracker {
    pub config: FlingConfig,
    samples: Vec<MovementSample>,
}

impl Default for FlingVelocityTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl FlingVelocityTracker {
    /// Creates a new velocity tracker with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: FlingConfig::default(),
            samples: Vec::with_capacity(16),
        }
    }

    /// Creates a new velocity tracker with custom configuration.
    #[must_use]
    pub fn with_config(config: FlingConfig) -> Self {
        Self {
            config,
            samples: Vec::with_capacity(16),
        }
    }

    /// Appends a new delta movement to the tracker.
    pub fn add_movement(&mut self, dx: f32, dy: f32, timestamp: Instant) {
        self.samples.push(MovementSample { dx, dy, timestamp });

        // Prune old samples exceeding window duration
        let window = self.config.window_duration;
        self.samples.retain(|s| {
            timestamp
                .checked_duration_since(s.timestamp)
                .is_some_and(|age| age <= window)
        });
    }

    /// Clears all recorded samples.
    pub fn reset(&mut self) {
        self.samples.clear();
    }

    /// Returns the number of active samples within the tracking window.
    #[must_use]
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    /// Computes the estimated fling velocity (px/sec) along (x, y) axes at time `now`.
    #[must_use]
    pub fn compute_velocity(&self, now: Instant) -> (f32, f32) {
        if self.samples.is_empty() {
            return (0.0, 0.0);
        }

        let last_sample = self.samples.last().unwrap();
        // If the pointer remained stationary prior to lift-off, no momentum is imparted.
        if let Some(idle_time) = now.checked_duration_since(last_sample.timestamp) {
            if idle_time > self.config.stationary_threshold {
                return (0.0, 0.0);
            }
        }

        let half_life_secs = self.config.decay_half_life.as_secs_f32().max(0.001);
        let lambda = std::f32::consts::LN_2 / half_life_secs;

        let mut total_weight = 0.0_f32;
        let mut weighted_vx = 0.0_f32;
        let mut weighted_vy = 0.0_f32;

        if self.samples.len() >= 2 {
            for i in 1..self.samples.len() {
                let prev = &self.samples[i - 1];
                let curr = &self.samples[i];
                let dt = curr
                    .timestamp
                    .checked_duration_since(prev.timestamp)
                    .map_or(0.016, |d| d.as_secs_f32())
                    .max(0.001);

                let inst_vx = curr.dx / dt;
                let inst_vy = curr.dy / dt;

                let age_secs = now
                    .checked_duration_since(curr.timestamp)
                    .map_or(0.0, |d| d.as_secs_f32());
                let weight = (-lambda * age_secs).exp();

                weighted_vx += inst_vx * weight;
                weighted_vy += inst_vy * weight;
                total_weight += weight;
            }
        } else {
            let single = &self.samples[0];
            let dt = now
                .checked_duration_since(single.timestamp)
                .map_or(0.016, |d| d.as_secs_f32())
                .max(0.008);
            weighted_vx = single.dx / dt;
            weighted_vy = single.dy / dt;
            total_weight = 1.0;
        }

        if total_weight < 1e-4 {
            return (0.0, 0.0);
        }

        let vx = weighted_vx / total_weight;
        let vy = weighted_vy / total_weight;

        let clamp_val = |v: f32| {
            if v.abs() < self.config.min_velocity {
                0.0
            } else {
                v.clamp(-self.config.max_velocity, self.config.max_velocity)
            }
        };

        (clamp_val(vx), clamp_val(vy))
    }
}

/// Computes the exact framerate-independent exponential momentum decay displacement and next velocity.
///
/// Returns `(displacement_step, new_velocity)`.
#[must_use]
pub fn step_momentum_decay(velocity: f32, friction: f32, dt: f32) -> (f32, f32) {
    if velocity.abs() < 0.5 || dt <= 0.0 {
        return (0.0, 0.0);
    }
    let lambda = friction.max(0.001);
    let decay = (-lambda * dt).exp();
    let step = (velocity / lambda) * (1.0 - decay);
    let next_vel = velocity * decay;
    (step, next_vel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fling_velocity_stationary_hold_cancels_velocity() {
        let mut tracker = FlingVelocityTracker::new();
        let start = Instant::now();

        tracker.add_movement(50.0, 0.0, start);
        tracker.add_movement(60.0, 0.0, start + Duration::from_millis(16));

        // Released 100ms later (exceeding 60ms stationary threshold)
        let release_time = start + Duration::from_millis(116);
        let (vx, vy) = tracker.compute_velocity(release_time);
        assert_eq!(vx, 0.0);
        assert_eq!(vy, 0.0);
    }

    #[test]
    fn test_fling_velocity_rapid_swipe_accuracy() {
        let mut tracker = FlingVelocityTracker::new();
        let start = Instant::now();

        // 400px swipe over 40ms -> ~10,000 px/s (clamped to max 8,000)
        tracker.add_movement(150.0, 0.0, start);
        tracker.add_movement(250.0, 0.0, start + Duration::from_millis(25));

        let (vx, _) = tracker.compute_velocity(start + Duration::from_millis(30));
        assert!(vx > 5000.0);
        assert!(vx <= 8000.0);
    }

    #[test]
    fn test_step_momentum_decay_invariance() {
        // Over 0.1 seconds, 1 step of 0.1s should equal continuous integration of 10 steps of 0.01s
        let initial_v = 1000.0;
        let friction = 5.0;

        let (step_single, v_end_single) = step_momentum_decay(initial_v, friction, 0.1);

        let mut v_multi = initial_v;
        let mut total_step_multi = 0.0;
        for _ in 0..10 {
            let (step, next_v) = step_momentum_decay(v_multi, friction, 0.01);
            total_step_multi += step;
            v_multi = next_v;
        }

        assert!((step_single - total_step_multi).abs() < 1e-2);
        assert!((v_end_single - v_multi).abs() < 1e-2);
    }
}
