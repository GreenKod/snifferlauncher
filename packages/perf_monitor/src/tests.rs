use super::constants::*;
use super::*;
use sniffer_core::vault::DataVault;
use sniffer_pkg::error::PackageError;
use sniffer_pkg::package::MemoryTrimLevel;
use std::sync::Arc;
use std::sync::atomic::Ordering;

fn make_vault() -> Arc<DataVault> {
    Arc::new(DataVault::default())
}

#[test]
fn query_get_cpu_usage_returns_valid_json() {
    let pkg = PerfMonitorPackage::new();
    let resp = pkg.query_service("getCpuUsage", "{}").unwrap();
    let v: serde_json::Value = serde_json::from_str(&resp).expect("must be valid JSON");
    assert_eq!(v["ok"], true);
    assert!(v["cpu_usage"].is_number());
}

#[test]
fn query_get_memory_usage_returns_valid_json() {
    let pkg = PerfMonitorPackage::new();
    let resp = pkg.query_service("getMemoryUsage", "{}").unwrap();
    let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(v["ok"], true);
    assert!(v["mem_mb"].is_number());
}

#[test]
fn query_get_all_metrics_returns_cpu_mem_fps() {
    let pkg = PerfMonitorPackage::new();
    let resp = pkg.query_service("getAllMetrics", "{}").unwrap();
    let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(v["ok"], true);
    assert!(v["cpu_usage"].is_number(), "must contain cpu_usage");
    assert!(v["mem_mb"].is_number(), "must contain mem_mb");
    assert!(v["fps"].is_number(), "must contain fps");
}

#[test]
fn query_set_polling_interval_clamps_and_echoes() {
    let pkg = PerfMonitorPackage::new();

    let resp = pkg
        .query_service("setPollingInterval", r#"{"interval_ms":800}"#)
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(v["interval_ms"], 800);
    assert_eq!(
        pkg.poll_interval_ms.load(Ordering::Relaxed),
        800,
        "atomic must reflect the new interval"
    );

    let _ = pkg.query_service("setPollingInterval", r#"{"interval_ms":5}"#);
    assert_eq!(
        pkg.poll_interval_ms.load(Ordering::Relaxed),
        POLL_INTERVAL_MIN_MS
    );

    let _ = pkg.query_service("setPollingInterval", r#"{"interval_ms":99999}"#);
    assert_eq!(
        pkg.poll_interval_ms.load(Ordering::Relaxed),
        POLL_INTERVAL_MAX_MS
    );
}

#[test]
fn query_set_polling_interval_missing_field_returns_error() {
    let pkg = PerfMonitorPackage::new();
    let err = pkg
        .query_service("setPollingInterval", r#"{"wrong_key":500}"#)
        .unwrap_err();
    assert!(
        matches!(err, PackageError::InvalidDescriptor(_)),
        "missing interval_ms must produce InvalidDescriptor"
    );
}

#[test]
fn query_unknown_method_returns_unsupported() {
    let pkg = PerfMonitorPackage::new();
    let err = pkg.query_service("frobnicateMetrics", "{}").unwrap_err();
    assert!(matches!(err, PackageError::UnsupportedMethod(_)));
}

#[test]
fn memory_trim_adjusts_poll_interval() {
    let pkg = PerfMonitorPackage::new();
    assert_eq!(
        pkg.poll_interval_ms.load(Ordering::Relaxed),
        POLL_INTERVAL_DEFAULT_MS,
        "must start at default interval"
    );

    pkg.on_memory_trim(MemoryTrimLevel::Moderate);
    assert_eq!(
        pkg.poll_interval_ms.load(Ordering::Relaxed),
        POLL_INTERVAL_MODERATE_MS
    );

    pkg.on_memory_trim(MemoryTrimLevel::Critical);
    assert_eq!(
        pkg.poll_interval_ms.load(Ordering::Relaxed),
        POLL_INTERVAL_CRITICAL_MS
    );

    pkg.on_memory_trim(MemoryTrimLevel::Emergency);
    assert_eq!(
        pkg.poll_interval_ms.load(Ordering::Relaxed),
        POLL_INTERVAL_EMERGENCY_MS
    );
}

#[test]
fn fps_ema_smooths_toward_new_rate() {
    let pkg = PerfMonitorPackage::new();
    let vault = make_vault();

    pkg.on_update(&vault, 1.0 / 60.0);
    let fps1 = pkg.read_fps();
    assert!((fps1 - 60.0).abs() < 0.1, "first frame seeds EMA: {fps1}");

    pkg.on_update(&vault, 1.0 / 30.0);
    let fps2 = pkg.read_fps();
    assert!(
        fps2 < 60.0 && fps2 > 30.0,
        "EMA must blend toward 30 fps: {fps2}"
    );
}

#[test]
fn fps_ema_ignores_zero_dt() {
    let pkg = PerfMonitorPackage::new();
    let vault = make_vault();

    pkg.on_update(&vault, 1.0 / 60.0);
    let before = pkg.read_fps();
    pkg.on_update(&vault, 0.0);
    let after = pkg.read_fps();
    assert_eq!(before, after, "dt=0 must be a no-op");
}

#[test]
fn init_and_unload_without_panic() {
    let mut pkg = PerfMonitorPackage::new();
    let vault = make_vault();

    assert!(pkg.on_init(vault).is_ok());
    std::thread::sleep(std::time::Duration::from_millis(50));
    pkg.on_unload();
    assert!(!pkg.running.load(Ordering::Relaxed));
}

#[test]
fn query_get_devkit_telemetry_returns_valid_json() {
    let pkg = PerfMonitorPackage::new();
    let profiler = Arc::new(std::sync::Mutex::new(
        sniffer_core::profiler::FrameProfiler::default(),
    ));
    pkg.set_profiler(profiler);

    let resp = pkg.query_service("getDevKitTelemetry", "{}").unwrap();
    let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(v["ok"], true);
    assert!(v["fps"].is_number());
    assert!(v["frame_time_ms"].is_number());
    assert!(v["touch"]["gesture"].is_string());
}
