use super::dedup::DeduplicatingLogger;
use super::formatter::{LogLevel, clean_message, clean_plugin_id, format_log_line};
use super::rate_limiter::ConsoleRateLimiter;
use std::time::Duration;

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
