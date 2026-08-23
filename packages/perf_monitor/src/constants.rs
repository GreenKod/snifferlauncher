use sniffer_pkg::package::MemoryTrimLevel;

pub const POLL_INTERVAL_DEFAULT_MS: u64 = 500;
pub const POLL_INTERVAL_MODERATE_MS: u64 = 1_000;
pub const POLL_INTERVAL_CRITICAL_MS: u64 = 2_000;
pub const POLL_INTERVAL_EMERGENCY_MS: u64 = 5_000;

pub const FPS_EMA_ALPHA: f32 = 0.1;
pub const POLL_INTERVAL_MIN_MS: u64 = 100;
pub const POLL_INTERVAL_MAX_MS: u64 = 10_000;

#[must_use]
pub fn trim_level_to_interval(level: MemoryTrimLevel) -> u64 {
    match level {
        MemoryTrimLevel::Moderate => POLL_INTERVAL_MODERATE_MS,
        MemoryTrimLevel::Critical => POLL_INTERVAL_CRITICAL_MS,
        MemoryTrimLevel::Emergency => POLL_INTERVAL_EMERGENCY_MS,
    }
}
