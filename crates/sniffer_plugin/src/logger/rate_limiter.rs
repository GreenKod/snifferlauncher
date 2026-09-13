use super::formatter::LogLevel;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub(crate) struct RateLimitBucket {
    pub(crate) window_start: Instant,
    pub(crate) emitted: u32,
    pub(crate) suppressed: u32,
}

/// Rate limiter applied exclusively to console/logcat output.
///
/// Prevents high-frequency polling or spam loops from flooding the console,
/// while allowing internal plugin execution, IPC calls, and DataVault operations
/// to proceed without any restriction.
pub struct ConsoleRateLimiter {
    enabled: AtomicBool,
    window: Duration,
    buckets: Mutex<HashMap<(String, LogLevel), RateLimitBucket>>,
}

impl Default for ConsoleRateLimiter {
    fn default() -> Self {
        Self::new(Duration::from_secs(1))
    }
}

impl ConsoleRateLimiter {
    #[must_use]
    pub fn new(window: Duration) -> Self {
        Self {
            enabled: AtomicBool::new(true),
            window,
            buckets: Mutex::new(HashMap::new()),
        }
    }

    /// Sets whether console rate limiting is active.
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
    }

    /// Returns whether console rate limiting is active.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Maximum logs allowed per window based on log level.
    #[must_use]
    pub fn max_per_window(level: LogLevel) -> u32 {
        match level {
            LogLevel::Info => 4,
            LogLevel::Warn => 8,
            LogLevel::Error => 16,
        }
    }

    /// Checks if a log line can be emitted to console.
    ///
    /// Returns `(allow_emit, previously_suppressed_count)`.
    pub fn check(&self, plugin: &str, level: LogLevel) -> (bool, u32) {
        if !self.enabled.load(Ordering::Relaxed) {
            return (true, 0);
        }

        let key = (plugin.to_string(), level);
        let max = Self::max_per_window(level);
        let now = Instant::now();

        if let Ok(mut map) = self.buckets.lock() {
            let bucket = map.entry(key).or_insert_with(|| RateLimitBucket {
                window_start: now,
                emitted: 0,
                suppressed: 0,
            });

            if now.duration_since(bucket.window_start) >= self.window {
                let prev_suppressed = bucket.suppressed;
                bucket.window_start = now;
                bucket.emitted = 1;
                bucket.suppressed = 0;
                (true, prev_suppressed)
            } else if bucket.emitted < max {
                bucket.emitted += 1;
                (true, 0)
            } else {
                bucket.suppressed = bucket.suppressed.saturating_add(1);
                (false, 0)
            }
        } else {
            (true, 0)
        }
    }

    /// Returns and resets all expired buckets that had suppressed logs.
    pub fn flush_suppressed(&self) -> Vec<(String, LogLevel, u32)> {
        let mut results = Vec::new();
        let now = Instant::now();
        if let Ok(mut map) = self.buckets.lock() {
            for ((plugin, level), bucket) in map.iter_mut() {
                if bucket.suppressed > 0 && now.duration_since(bucket.window_start) >= self.window {
                    results.push((plugin.clone(), *level, bucket.suppressed));
                    bucket.suppressed = 0;
                    bucket.window_start = now;
                    bucket.emitted = 0;
                }
            }
        }
        results
    }
}
