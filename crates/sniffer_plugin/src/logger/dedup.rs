use super::formatter::{LogLevel, clean_message, clean_plugin_id, format_log_line};
use super::rate_limiter::ConsoleRateLimiter;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Writes the formatted log line to the appropriate platform backend.
pub(crate) fn output_line(level: LogLevel, line: &str) {
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
pub(crate) struct Entry {
    pub(crate) plugin: String,
    pub(crate) level: LogLevel,
    pub(crate) message: String,
    pub(crate) count: u32,
    pub(crate) first_seen: Instant,
    pub(crate) last_seen: Instant,
}

/// Thread-safe logger that deduplicates repeated plugin logs with an `[xN]` counter.
pub struct DeduplicatingLogger {
    pub(crate) entries: Mutex<HashMap<(String, LogLevel, String), Entry>>,
    pub(crate) last_msg_by_plugin: Mutex<HashMap<String, (LogLevel, String)>>,
    flusher_started: AtomicBool,
    pub rate_limiter: ConsoleRateLimiter,
}

impl Default for DeduplicatingLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl DeduplicatingLogger {
    #[must_use]
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
                        super::LOGGER
                            .flush_stale(Duration::from_millis(150), Duration::from_secs(1));
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
