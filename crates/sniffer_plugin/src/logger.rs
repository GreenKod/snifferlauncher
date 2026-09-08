use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

/// Severity levels for plugin logs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

/// Normalizes raw plugin identifiers into short, readable names.
///
/// For example:
/// - `"com.sniffer.defaultui"` -> `"default_ui"`
/// - `"com.sniffer.clockwidget"` -> `"clock_widget"`
/// - `"com.sniffer.devkithud"` -> `"devkit_hud"`
/// - `"com.sniffer.dock"` -> `"dock"`
/// - `"com.sniffer.logger"` -> `"logger_plugin"`
pub fn clean_plugin_id(id: &str) -> String {
    let trimmed = id.trim();
    if trimmed.is_empty() {
        return "system".to_string();
    }
    let stripped = trimmed.strip_prefix("com.sniffer.").unwrap_or(trimmed);
    match stripped {
        "defaultui" | "default_ui" => "default_ui".to_string(),
        "clockwidget" | "clock_widget" => "clock_widget".to_string(),
        "devkithud" | "devkit_hud" => "devkit_hud".to_string(),
        "dock" => "dock".to_string(),
        "logger" | "logger_plugin" => "logger_plugin".to_string(),
        other => {
            if let Some(pos) = other.rfind('.') {
                other[pos + 1..].to_string()
            } else {
                other.to_string()
            }
        }
    }
}

/// Cleans redundant prefix tags often emitted by JS plugins (e.g. `[Dock Plugin] `, `[Clock Widget] `)
/// while safely preserving legitimate data structures like JSON arrays (`[1, 2, 3]`).
pub fn clean_message(msg: &str) -> String {
    let trimmed = msg.trim();
    if let Some(rest) = trimmed.strip_prefix('[') {
        if let Some(close_idx) = rest.find(']') {
            let inside = &rest[..close_idx];
            let after = rest[close_idx + 1..].trim_start();
            // Match typical log tags: alphanumeric with spaces, dashes, underscores, colons, or @ symbols,
            // containing at least one alphabetic character, followed by non-empty message content.
            if !after.is_empty()
                && inside.chars().all(|c| {
                    c.is_alphanumeric()
                        || c.is_whitespace()
                        || c == '-'
                        || c == '_'
                        || c == ':'
                        || c == '@'
                })
                && inside.chars().any(char::is_alphabetic)
            {
                let cleaned = after.strip_prefix(':').unwrap_or(after).trim_start();
                return cleaned.to_string();
            }
        }
    }
    trimmed.to_string()
}

/// Formats a single log line according to the user specification:
/// - Info: `# <message> from <plugin>, type info [xN]` or `# info from <plugin> [xN]`
/// - Warn: `# warn: <message> from <plugin> [xN]` or `# warn from <plugin> [xN]`
/// - Error: `# error: <message> from <plugin> [xN]` or `# error from <plugin> [xN]`
pub fn format_log_line(plugin: &str, level: LogLevel, message: &str, count: u32) -> String {
    let clean_p = clean_plugin_id(plugin);
    let clean_m = clean_message(message);

    let repeat_suffix = if count > 1 {
        format!(" [x{count}]")
    } else {
        String::new()
    };

    match level {
        LogLevel::Info => {
            if clean_m.is_empty() || clean_m.eq_ignore_ascii_case("info") {
                format!("# info from {clean_p}{repeat_suffix}")
            } else {
                format!("# {clean_m} from {clean_p}, type info{repeat_suffix}")
            }
        }
        LogLevel::Warn => {
            if clean_m.is_empty()
                || clean_m.eq_ignore_ascii_case("warn")
                || clean_m.eq_ignore_ascii_case("warning")
            {
                format!("# warn from {clean_p}{repeat_suffix}")
            } else {
                format!("# warn: {clean_m} from {clean_p}{repeat_suffix}")
            }
        }
        LogLevel::Error => {
            if clean_m.is_empty() || clean_m.eq_ignore_ascii_case("error") {
                format!("# error from {clean_p}{repeat_suffix}")
            } else {
                format!("# error: {clean_m} from {clean_p}{repeat_suffix}")
            }
        }
    }
}

/// Writes the formatted log line to the appropriate platform backend.
fn output_line(level: LogLevel, line: &str) {
    #[cfg(target_os = "android")]
    {
        let prio = match level {
            LogLevel::Info => 4,  // ANDROID_LOG_INFO
            LogLevel::Warn => 5,  // ANDROID_LOG_WARN
            LogLevel::Error => 6, // ANDROID_LOG_ERROR
        };
        sniffer_core::log::android_log(prio, line);
    }
    #[cfg(not(target_os = "android"))]
    {
        match level {
            LogLevel::Error => eprintln!("{line}"),
            _ => println!("{line}"),
        }
    }
}

#[derive(Debug, Clone)]
struct RateLimitBucket {
    window_start: Instant,
    emitted: u32,
    suppressed: u32,
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

#[derive(Debug, Clone)]
struct Entry {
    plugin: String,
    level: LogLevel,
    message: String,
    count: u32,
    first_seen: Instant,
    last_seen: Instant,
}

/// Thread-safe logger that deduplicates repeated plugin logs with an `[xN]` counter.
pub struct DeduplicatingLogger {
    entries: Mutex<HashMap<(String, LogLevel, String), Entry>>,
    last_msg_by_plugin: Mutex<HashMap<String, (LogLevel, String)>>,
    flusher_started: AtomicBool,
    pub rate_limiter: ConsoleRateLimiter,
}

impl Default for DeduplicatingLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl DeduplicatingLogger {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            last_msg_by_plugin: Mutex::new(HashMap::new()),
            flusher_started: AtomicBool::new(false),
            rate_limiter: ConsoleRateLimiter::default(),
        }
    }

    fn emit_to_console(&self, plugin: &str, level: LogLevel, line: &str) {
        let (allow, prev_suppressed) = self.rate_limiter.check(plugin, level);

        if prev_suppressed > 0 {
            let level_name = match level {
                LogLevel::Info => "info",
                LogLevel::Warn => "warn",
                LogLevel::Error => "error",
            };
            let notice = format!(
                "# [rate limit] {prev_suppressed} {level_name} logs suppressed from {plugin}"
            );
            output_line(level, &notice);
        }

        if allow {
            output_line(level, line);
        }
    }

    fn ensure_flusher(&self) {
        if !self.flusher_started.swap(true, Ordering::SeqCst) {
            std::thread::Builder::new()
                .name("sniffer-log-flusher".to_string())
                .spawn(move || {
                    loop {
                        std::thread::sleep(Duration::from_millis(50));
                        LOGGER.flush_stale(Duration::from_millis(150), Duration::from_secs(1));
                    }
                })
                .ok();
        }
    }

    /// Logs a message from a plugin with deduplication.
    pub fn log(&self, plugin_id: &str, level: LogLevel, raw_msg: &str) {
        self.ensure_flusher();

        let plugin = clean_plugin_id(plugin_id);
        let message = clean_message(raw_msg);
        let key = (plugin.clone(), level, message.clone());
        let now = Instant::now();

        // 1. If this plugin previously logged a different message, flush that previous message immediately.
        let mut immediate_flush: Option<Entry> = None;
        {
            if let Ok(mut last_map) = self.last_msg_by_plugin.lock() {
                if let Some((prev_lvl, prev_msg)) = last_map.get(&plugin) {
                    if (*prev_lvl, prev_msg.as_str()) != (level, message.as_str()) {
                        let prev_key = (plugin.clone(), *prev_lvl, prev_msg.clone());
                        if let Ok(mut entries) = self.entries.lock() {
                            if let Some(entry) = entries.remove(&prev_key) {
                                immediate_flush = Some(entry);
                            }
                        }
                    }
                }
                last_map.insert(plugin.clone(), (level, message.clone()));
            }
        }

        if let Some(prev_entry) = immediate_flush {
            let line = format_log_line(
                &prev_entry.plugin,
                prev_entry.level,
                &prev_entry.message,
                prev_entry.count,
            );
            self.emit_to_console(&prev_entry.plugin, prev_entry.level, &line);
        }

        // 2. Accumulate current message or increment repetition counter
        let mut overflow_flush: Option<Entry> = None;
        {
            if let Ok(mut entries) = self.entries.lock() {
                if let Some(entry) = entries.get_mut(&key) {
                    entry.count = entry.count.saturating_add(1);
                    entry.last_seen = now;
                    // Max window reached: flush batch if it has been repeating continuously for >1s
                    if entry.first_seen.elapsed() >= Duration::from_secs(1) {
                        overflow_flush = Some(entry.clone());
                        entry.count = 0;
                        entry.first_seen = now;
                    }
                } else {
                    // Prevent memory unbounded growth
                    if entries.len() >= 128 {
                        // Find and remove oldest
                        if let Some(oldest_key) = entries
                            .iter()
                            .min_by_key(|(_, e)| e.last_seen)
                            .map(|(k, _)| k.clone())
                        {
                            if let Some(old_entry) = entries.remove(&oldest_key) {
                                overflow_flush = Some(old_entry);
                            }
                        }
                    }
                    entries.insert(
                        key,
                        Entry {
                            plugin,
                            level,
                            message,
                            count: 1,
                            first_seen: now,
                            last_seen: now,
                        },
                    );
                }
            }
        }

        if let Some(overflow_entry) = overflow_flush {
            if overflow_entry.count > 0 {
                let line = format_log_line(
                    &overflow_entry.plugin,
                    overflow_entry.level,
                    &overflow_entry.message,
                    overflow_entry.count,
                );
                self.emit_to_console(&overflow_entry.plugin, overflow_entry.level, &line);
            }
        }
    }

    /// Flushes entries that are older than `stale_duration` or have exceeded `max_duration`.
    pub fn flush_stale(&self, stale_duration: Duration, max_duration: Duration) {
        let mut to_flush = Vec::new();
        let now = Instant::now();

        if let Ok(mut entries) = self.entries.lock() {
            let mut keys_to_remove = Vec::new();
            for (key, entry) in entries.iter_mut() {
                if entry.count > 0
                    && (now.duration_since(entry.last_seen) >= stale_duration
                        || now.duration_since(entry.first_seen) >= max_duration)
                {
                    to_flush.push(entry.clone());
                    keys_to_remove.push(key.clone());
                }
            }
            for key in keys_to_remove {
                entries.remove(&key);
            }
        }

        for entry in to_flush {
            let line = format_log_line(&entry.plugin, entry.level, &entry.message, entry.count);
            self.emit_to_console(&entry.plugin, entry.level, &line);
        }

        // Flush expired rate-limit suppression notices
        for (plugin, level, suppressed_count) in self.rate_limiter.flush_suppressed() {
            let level_name = match level {
                LogLevel::Info => "info",
                LogLevel::Warn => "warn",
                LogLevel::Error => "error",
            };
            let notice = format!(
                "# [rate limit] {suppressed_count} {level_name} logs suppressed from {plugin}"
            );
            output_line(level, &notice);
        }
    }

    /// Flushes all pending log messages immediately.
    pub fn flush_all(&self) {
        let mut to_flush = Vec::new();
        if let Ok(mut entries) = self.entries.lock() {
            for (_, entry) in entries.drain() {
                if entry.count > 0 {
                    to_flush.push(entry);
                }
            }
        }
        for entry in to_flush {
            let line = format_log_line(&entry.plugin, entry.level, &entry.message, entry.count);
            output_line(entry.level, &line);
        }
    }
}

pub static LOGGER: LazyLock<DeduplicatingLogger> = LazyLock::new(DeduplicatingLogger::new);

/// Public logging functions.
pub fn log(plugin: &str, level: LogLevel, message: &str) {
    LOGGER.log(plugin, level, message);
}

pub fn info(plugin: &str, message: &str) {
    LOGGER.log(plugin, LogLevel::Info, message);
}

pub fn warn(plugin: &str, message: &str) {
    LOGGER.log(plugin, LogLevel::Warn, message);
}

pub fn error(plugin: &str, message: &str) {
    LOGGER.log(plugin, LogLevel::Error, message);
}

pub fn flush() {
    LOGGER.flush_all();
}

pub fn set_console_rate_limiting(enabled: bool) {
    LOGGER.rate_limiter.set_enabled(enabled);
}

#[must_use]
pub fn is_console_rate_limiting_enabled() -> bool {
    LOGGER.rate_limiter.is_enabled()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_plugin_id() {
        assert_eq!(clean_plugin_id("com.sniffer.defaultui"), "default_ui");
        assert_eq!(clean_plugin_id("default_ui"), "default_ui");
        assert_eq!(clean_plugin_id("com.sniffer.clockwidget"), "clock_widget");
        assert_eq!(clean_plugin_id("clock_widget"), "clock_widget");
        assert_eq!(clean_plugin_id("com.sniffer.devkithud"), "devkit_hud");
        assert_eq!(clean_plugin_id("devkit_hud"), "devkit_hud");
        assert_eq!(clean_plugin_id("com.sniffer.dock"), "dock");
        assert_eq!(clean_plugin_id("dock"), "dock");
        assert_eq!(clean_plugin_id("com.sniffer.logger"), "logger_plugin");
        assert_eq!(clean_plugin_id("logger_plugin"), "logger_plugin");
        assert_eq!(clean_plugin_id("org.custom.my_plugin"), "my_plugin");
        assert_eq!(clean_plugin_id(""), "system");
        assert_eq!(clean_plugin_id("   "), "system");
    }

    #[test]
    fn test_clean_message() {
        assert_eq!(
            clean_message("[Dock Plugin] Initializing Clean Android Dock plugin..."),
            "Initializing Clean Android Dock plugin..."
        );
        assert_eq!(
            clean_message("[Clock Widget] Started and registering APIs..."),
            "Started and registering APIs..."
        );
        assert_eq!(
            clean_message("[SnifferLauncher JS] Click received for ID: test"),
            "Click received for ID: test"
        );
        assert_eq!(
            clean_message("[LOG - INFO @ 12:00:00]: Application ready"),
            "Application ready"
        );
        // Ensure JSON arrays are not stripped
        assert_eq!(clean_message("[1, 2, 3]"), "[1, 2, 3]");
        assert_eq!(clean_message(r#"[{"id":"test"}]"#), r#"[{"id":"test"}]"#);
        // Regular messages remain intact
        assert_eq!(clean_message("merhaba dünya"), "merhaba dünya");
    }

    #[test]
    fn test_format_log_line() {
        // Info with message
        assert_eq!(
            format_log_line("default_ui", LogLevel::Info, "merhaba dünya", 1),
            "# merhaba dünya from default_ui, type info"
        );
        assert_eq!(
            format_log_line("default_ui", LogLevel::Info, "merhaba dünya", 5),
            "# merhaba dünya from default_ui, type info [x5]"
        );

        // Info without message
        assert_eq!(
            format_log_line("default_ui", LogLevel::Info, "", 1),
            "# info from default_ui"
        );
        assert_eq!(
            format_log_line("default_ui", LogLevel::Info, "info", 5),
            "# info from default_ui [x5]"
        );

        // Warn
        assert_eq!(
            format_log_line("dock", LogLevel::Warn, "low battery", 1),
            "# warn: low battery from dock"
        );
        assert_eq!(
            format_log_line("dock", LogLevel::Warn, "low battery", 3),
            "# warn: low battery from dock [x3]"
        );

        // Error
        assert_eq!(
            format_log_line("clock_widget", LogLevel::Error, "failed to parse time", 1),
            "# error: failed to parse time from clock_widget"
        );
        assert_eq!(
            format_log_line("clock_widget", LogLevel::Error, "failed to parse time", 7),
            "# error: failed to parse time from clock_widget [x7]"
        );

        // Automatic ID cleanup
        assert_eq!(
            format_log_line("com.sniffer.defaultui", LogLevel::Info, "test", 5),
            "# test from default_ui, type info [x5]"
        );
    }

    #[test]
    fn test_deduplicating_logger_counts() {
        let logger = DeduplicatingLogger::new();
        // Log the same message 5 times
        for _ in 0..5 {
            logger.log("com.sniffer.defaultui", LogLevel::Info, "test repetition");
        }

        // Check internal entry count
        let entries = logger.entries.lock().unwrap();
        let key = (
            "default_ui".to_string(),
            LogLevel::Info,
            "test repetition".to_string(),
        );
        let entry = entries.get(&key).expect("entry must exist");
        assert_eq!(entry.count, 5);
        assert_eq!(entry.plugin, "default_ui");
        assert_eq!(entry.message, "test repetition");
        assert_eq!(entry.level, LogLevel::Info);
    }

    #[test]
    fn test_deduplicating_logger_flush_on_different_message() {
        let logger = DeduplicatingLogger::new();
        // Log message A 3 times
        for _ in 0..3 {
            logger.log("dock", LogLevel::Info, "Message A");
        }
        // Log message B once: this should trigger immediate flush of Message A
        logger.log("dock", LogLevel::Info, "Message B");

        let entries = logger.entries.lock().unwrap();
        // Message A should have been removed from entries (flushed)
        let key_a = ("dock".to_string(), LogLevel::Info, "Message A".to_string());
        assert!(entries.get(&key_a).is_none());

        // Message B should now be the pending entry with count = 1
        let key_b = ("dock".to_string(), LogLevel::Info, "Message B".to_string());
        let entry_b = entries.get(&key_b).expect("entry B must exist");
        assert_eq!(entry_b.count, 1);
    }

    #[test]
    fn test_console_rate_limiter() {
        let limiter = ConsoleRateLimiter::new(Duration::from_millis(50));
        let max_info = ConsoleRateLimiter::max_per_window(LogLevel::Info); // 4

        // First max_info logs are allowed
        for _ in 0..max_info {
            let (allow, suppressed) = limiter.check("default_ui", LogLevel::Info);
            assert!(allow);
            assert_eq!(suppressed, 0);
        }

        // Next 3 logs within same window are suppressed
        for _ in 0..3 {
            let (allow, suppressed) = limiter.check("default_ui", LogLevel::Info);
            assert!(!allow);
            assert_eq!(suppressed, 0);
        }

        // Sleep to let window expire
        std::thread::sleep(Duration::from_millis(60));

        // Next log in new window is allowed and reports 3 suppressed logs
        let (allow, suppressed) = limiter.check("default_ui", LogLevel::Info);
        assert!(allow);
        assert_eq!(suppressed, 3);
    }

    #[test]
    fn test_rate_limiter_level_isolation() {
        let limiter = ConsoleRateLimiter::new(Duration::from_millis(50));
        let max_info = ConsoleRateLimiter::max_per_window(LogLevel::Info);

        // Exhaust info limit
        for _ in 0..max_info {
            let (allow, _) = limiter.check("default_ui", LogLevel::Info);
            assert!(allow);
        }
        let (allow_info, _) = limiter.check("default_ui", LogLevel::Info);
        assert!(!allow_info); // Info is now throttled

        // Errors should NOT be throttled by info spam
        let (allow_err, _) = limiter.check("default_ui", LogLevel::Error);
        assert!(allow_err);
    }
}
