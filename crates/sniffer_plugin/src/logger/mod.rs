pub mod dedup;
pub mod formatter;
pub mod rate_limiter;

#[cfg(test)]
mod tests;

pub use dedup::DeduplicatingLogger;
pub use formatter::{LogLevel, clean_message, clean_plugin_id, format_log_line};
pub use rate_limiter::ConsoleRateLimiter;

use std::sync::LazyLock;

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
