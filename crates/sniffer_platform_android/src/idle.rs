use std::time::{Duration, Instant};

/// Default inactivity duration before transitioning to idle state (500 milliseconds).
pub const DEFAULT_IDLE_THRESHOLD: Duration = Duration::from_millis(500);

/// Event polling timeout when the application is idle and waiting for user input or events (100 milliseconds).
pub const IDLE_POLL_TIMEOUT: Duration = Duration::from_millis(100);
/// Event polling timeout during active animations, dragging, or kinetic physics (0 milliseconds).
pub const ACTIVE_POLL_TIMEOUT: Duration = Duration::from_millis(0);
/// Event polling timeout during normal paced rendering without active animations (8 milliseconds).
pub const PACED_POLL_TIMEOUT: Duration = Duration::from_millis(8);

/// Target frame pacing duration when the application is idle (100 milliseconds, minimizing wakeups).
pub const IDLE_FRAME_DURATION: Duration = Duration::from_millis(100);
/// Target frame pacing duration during active physics, animations, or dragging (8 milliseconds, ~120 FPS).
pub const ACTIVE_FRAME_DURATION: Duration = Duration::from_millis(8);
/// Target frame pacing duration during standard active UI interaction (16 milliseconds, ~60 FPS).
pub const STANDARD_FRAME_DURATION: Duration = Duration::from_millis(16);

/// Tracks user interaction timestamps and evaluates whether the application is in an idle state.
#[derive(Debug, Clone)]
pub struct IdleDetector {
    /// Flag indicating whether the application has entered the idle state.
    is_idle: bool,
    /// Timestamp of the last recorded user interaction or active system work.
    last_interaction_time: Instant,
    /// Inactivity threshold duration required to enter the idle state.
    idle_threshold: Duration,
}

impl Default for IdleDetector {
    fn default() -> Self {
        Self::new(DEFAULT_IDLE_THRESHOLD)
    }
}

impl IdleDetector {
    /// Creates a new `IdleDetector` with a custom inactivity threshold duration.
    #[must_use]
    pub fn new(idle_threshold: Duration) -> Self {
        Self {
            is_idle: false,
            last_interaction_time: Instant::now(),
            idle_threshold,
        }
    }

    /// Marks user or system activity, immediately resetting the idle flag
    /// and updating the interaction timestamp.
    pub fn mark_interaction(&mut self) {
        self.is_idle = false;
        self.last_interaction_time = Instant::now();
    }

    /// Returns `true` if the application has been determined to be idle.
    #[must_use]
    pub fn is_idle(&self) -> bool {
        self.is_idle
    }

    /// Returns the duration elapsed since the last recorded interaction or activity.
    #[must_use]
    pub fn time_since_last_interaction(&self) -> Duration {
        self.last_interaction_time.elapsed()
    }

    /// Returns the timestamp of the last recorded interaction or activity.
    #[must_use]
    pub fn last_interaction_time(&self) -> Instant {
        self.last_interaction_time
    }

    /// Returns the configured idle threshold duration.
    #[must_use]
    pub fn idle_threshold(&self) -> Duration {
        self.idle_threshold
    }

    /// Updates the configured idle threshold duration.
    pub fn set_idle_threshold(&mut self, threshold: Duration) {
        self.idle_threshold = threshold;
    }

    /// Evaluates and updates the idle state based on active work and elapsed time.
    ///
    /// - If `has_active_work` is `true` (e.g. active animations, physics, user dragging, pending actions),
    ///   the detector marks activity, keeping `is_idle = false`.
    /// - If `has_active_work` is `false` and elapsed time since last interaction exceeds `idle_threshold`,
    ///   `is_idle` transitions to `true`.
    /// - Otherwise, `is_idle` remains `false`.
    ///
    /// Returns the updated `is_idle` boolean flag.
    pub fn update(&mut self, has_active_work: bool) -> bool {
        if has_active_work {
            self.is_idle = false;
            self.last_interaction_time = Instant::now();
        } else {
            self.is_idle = self.last_interaction_time.elapsed() >= self.idle_threshold;
        }
        self.is_idle
    }

    /// Returns the recommended event polling timeout based on current idle state and animations:
    /// - `ACTIVE_POLL_TIMEOUT` (0ms) when active animations, dragging, or kinetic physics are in progress.
    /// - `IDLE_POLL_TIMEOUT` (100ms) when the application is in an idle state.
    /// - `PACED_POLL_TIMEOUT` (8ms) during non-idle pacing without active animations.
    #[must_use]
    pub fn recommended_poll_timeout(&self, has_active_animation: bool) -> Duration {
        if has_active_animation {
            ACTIVE_POLL_TIMEOUT
        } else if self.is_idle {
            IDLE_POLL_TIMEOUT
        } else {
            PACED_POLL_TIMEOUT
        }
    }

    /// Returns the target frame pacing duration based on current idle state and active animations:
    /// - `ACTIVE_FRAME_DURATION` (8ms, ~120 FPS) when physics, drag, or transitions are active.
    /// - `IDLE_FRAME_DURATION` (100ms, ~10 FPS) when the application is in an idle state.
    /// - `STANDARD_FRAME_DURATION` (16ms, ~60 FPS) when active without high-refresh animations.
    #[must_use]
    pub fn target_frame_duration(&self, has_active_animation: bool) -> Duration {
        if has_active_animation {
            ACTIVE_FRAME_DURATION
        } else if self.is_idle {
            IDLE_FRAME_DURATION
        } else {
            STANDARD_FRAME_DURATION
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_target_frame_duration() {
        let mut detector = IdleDetector::new(Duration::from_millis(15));
        // Not idle, standard state -> STANDARD_FRAME_DURATION (16ms)
        assert_eq!(
            detector.target_frame_duration(false),
            STANDARD_FRAME_DURATION
        );

        // Not idle, active animation -> ACTIVE_FRAME_DURATION (8ms)
        assert_eq!(detector.target_frame_duration(true), ACTIVE_FRAME_DURATION);

        // Transition to idle
        sleep(Duration::from_millis(25));
        detector.update(false);
        assert!(detector.is_idle());

        // Idle, no animation -> IDLE_FRAME_DURATION (100ms)
        assert_eq!(detector.target_frame_duration(false), IDLE_FRAME_DURATION);

        // Idle, but active animation flag passed -> ACTIVE_FRAME_DURATION (8ms)
        assert_eq!(detector.target_frame_duration(true), ACTIVE_FRAME_DURATION);
    }

    #[test]
    fn test_recommended_poll_timeout() {
        let mut detector = IdleDetector::new(Duration::from_millis(15));
        // Not idle, no active animation -> PACED_POLL_TIMEOUT (8ms)
        assert_eq!(detector.recommended_poll_timeout(false), PACED_POLL_TIMEOUT);

        // Not idle, has active animation -> ACTIVE_POLL_TIMEOUT (0ms)
        assert_eq!(detector.recommended_poll_timeout(true), ACTIVE_POLL_TIMEOUT);

        // Transition to idle
        sleep(Duration::from_millis(25));
        detector.update(false);
        assert!(detector.is_idle());

        // Idle, no active animation -> IDLE_POLL_TIMEOUT (100ms)
        assert_eq!(detector.recommended_poll_timeout(false), IDLE_POLL_TIMEOUT);

        // Idle, but active animation flag passed -> ACTIVE_POLL_TIMEOUT (0ms)
        assert_eq!(detector.recommended_poll_timeout(true), ACTIVE_POLL_TIMEOUT);
    }

    #[test]
    fn test_idle_detector_initial_state() {
        let detector = IdleDetector::default();
        assert!(!detector.is_idle());
        assert_eq!(detector.idle_threshold(), DEFAULT_IDLE_THRESHOLD);
        assert!(detector.time_since_last_interaction() < Duration::from_millis(50));
    }

    #[test]
    fn test_idle_detector_transitions_to_idle_after_threshold() {
        let mut detector = IdleDetector::new(Duration::from_millis(15));
        assert!(!detector.is_idle());

        // Immediately updating with no active work should not be idle yet
        assert!(!detector.update(false));
        assert!(!detector.is_idle());

        // Wait past threshold
        sleep(Duration::from_millis(25));
        assert!(detector.update(false));
        assert!(detector.is_idle());
        assert!(detector.time_since_last_interaction() >= Duration::from_millis(15));
    }

    #[test]
    fn test_idle_detector_active_work_prevents_idle() {
        let mut detector = IdleDetector::new(Duration::from_millis(15));
        sleep(Duration::from_millis(25));

        // Active work should reset timestamp and keep idle false
        assert!(!detector.update(true));
        assert!(!detector.is_idle());
        assert!(detector.time_since_last_interaction() < Duration::from_millis(10));
    }

    #[test]
    fn test_idle_detector_mark_interaction_resets_idle() {
        let mut detector = IdleDetector::new(Duration::from_millis(15));
        sleep(Duration::from_millis(25));
        detector.update(false);
        assert!(detector.is_idle());

        // Mark interaction wakes system up immediately
        detector.mark_interaction();
        assert!(!detector.is_idle());
        assert!(detector.time_since_last_interaction() < Duration::from_millis(10));
    }

    #[test]
    fn test_idle_detector_custom_threshold() {
        let mut detector = IdleDetector::default();
        detector.set_idle_threshold(Duration::from_millis(200));
        assert_eq!(detector.idle_threshold(), Duration::from_millis(200));
    }
}
