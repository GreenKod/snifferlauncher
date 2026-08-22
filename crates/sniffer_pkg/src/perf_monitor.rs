//! `PerfMonitorPackage` — CPU / RAM / FPS metric service.
//!
//! Provides system performance metrics to the rest of the application via
//! two channels:
//!
//! 1. **[`DataVault`][sniffer_core::vault::DataVault] writes** — background
//!    thread pushes fresh values every `poll_interval_ms` milliseconds under
//!    the following keys:
//!    - `pkg.perf.cpu_usage` — global CPU load, 0–100 (string)
//!    - `pkg.perf.mem_mb`    — used RAM in MiB (string)
//!
//! 2. **`query_service` bridge** — JS can call
//!    `host_pkg_query("com.sniffer.perf", method, payload)` with:
//!    - `"getCpuUsage"`        → `{"ok":true,"cpu_usage":<f>}`
//!    - `"getMemoryUsage"`     → `{"ok":true,"mem_mb":<f>}`
//!    - `"getAllMetrics"`      → `{"ok":true,"cpu_usage":<f>,"mem_mb":<f>,"fps":<f>}`
//!    - `"setPollingInterval"` → `{"ok":true,"interval_ms":<n>}` (100–10 000 ms)
//!
//! FPS is tracked on the **main thread** via [`LauncherPackage::on_update`]
//! using an exponential moving average (α = 0.1).

use crate::error::PackageError;
use crate::package::{LauncherPackage, MemoryTrimLevel, PackageKind, PackageMeta};
use sniffer_core::vault::DataVault;
// sysinfo 0.32: all methods are direct on System / Cpu structs; no trait imports needed.
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
};
use std::thread;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Default background polling interval (ms).
const POLL_INTERVAL_DEFAULT_MS: u64 = 500;
/// Polling interval used when [`MemoryTrimLevel::Moderate`] is signalled.
const POLL_INTERVAL_MODERATE_MS: u64 = 1_000;
/// Polling interval used when [`MemoryTrimLevel::Critical`] is signalled.
const POLL_INTERVAL_CRITICAL_MS: u64 = 2_000;
/// Polling interval used when [`MemoryTrimLevel::Emergency`] is signalled.
const POLL_INTERVAL_EMERGENCY_MS: u64 = 5_000;

/// Smoothing factor for the FPS exponential moving average (0 < α ≤ 1).
const FPS_EMA_ALPHA: f32 = 0.1;

/// Minimum sane polling interval that `setPollingInterval` will accept (ms).
const POLL_INTERVAL_MIN_MS: u64 = 100;
/// Maximum sane polling interval that `setPollingInterval` will accept (ms).
const POLL_INTERVAL_MAX_MS: u64 = 10_000;

// ---------------------------------------------------------------------------
// PerfMonitorPackage
// ---------------------------------------------------------------------------

/// Background service that collects CPU, RAM, and FPS metrics.
///
/// ## Usage
///
/// ```rust,ignore
/// let mut registry = PackageRegistry::new(vault.clone());
/// registry.register_service(Arc::new(PerfMonitorPackage::new()));
/// ```
///
/// After registration, JavaScript can read live values via:
/// ```js
/// host_vault_get("pkg.perf.cpu_usage") // "42.3"
/// host_vault_get("pkg.perf.mem_mb")    // "1024"
/// host_pkg_query("com.sniffer.perf", "getAllMetrics", "{}")
/// ```
pub struct PerfMonitorPackage {
    meta: PackageMeta,
    /// Set to `false` in `on_unload` to stop the background thread.
    running: Arc<AtomicBool>,
    /// Background polling interval in milliseconds; adjusted by `MemoryTrimLevel`.
    poll_interval_ms: Arc<AtomicU64>,
    /// Latest CPU usage percentage (0–100) stored as `f32` bits.
    cpu_usage_bits: Arc<AtomicU32>,
    /// Latest used RAM in MiB stored as `f32` bits.
    mem_mb_bits: Arc<AtomicU32>,
    /// Exponential moving average of frame rate stored as `f32` bits.
    /// Updated on the main thread via `on_update`.
    fps_ema_bits: Arc<AtomicU32>,
    /// Background thread handle.  Taken and joined in `on_unload`.
    thread_handle: Mutex<Option<thread::JoinHandle<()>>>,
}

impl PerfMonitorPackage {
    /// Create a new, uninitialised `PerfMonitorPackage`.
    ///
    /// Call [`PackageRegistry::register_service`][crate::registry::PackageRegistry::register_service]
    /// to initialise and start the background thread.
    #[must_use]
    pub fn new() -> Self {
        Self {
            meta: PackageMeta {
                id: "com.sniffer.perf",
                version: (0, 1, 0),
                kind: PackageKind::Service,
            },
            running: Arc::new(AtomicBool::new(false)),
            poll_interval_ms: Arc::new(AtomicU64::new(POLL_INTERVAL_DEFAULT_MS)),
            cpu_usage_bits: Arc::new(AtomicU32::new(0_f32.to_bits())),
            mem_mb_bits: Arc::new(AtomicU32::new(0_f32.to_bits())),
            fps_ema_bits: Arc::new(AtomicU32::new(0_f32.to_bits())),
            thread_handle: Mutex::new(None),
        }
    }

    // ── Private readers (used by query_service) ──────────────────────────

    fn read_cpu(&self) -> f32 {
        f32::from_bits(self.cpu_usage_bits.load(Ordering::Relaxed))
    }

    fn read_mem_mb(&self) -> f32 {
        f32::from_bits(self.mem_mb_bits.load(Ordering::Relaxed))
    }

    fn read_fps(&self) -> f32 {
        f32::from_bits(self.fps_ema_bits.load(Ordering::Relaxed))
    }
}

impl Default for PerfMonitorPackage {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// LauncherPackage implementation
// ---------------------------------------------------------------------------

impl LauncherPackage for PerfMonitorPackage {
    fn meta(&self) -> &PackageMeta {
        &self.meta
    }

    /// Spawns the background polling thread.
    ///
    /// The thread calls `sysinfo::System::refresh_cpu_usage()` and
    /// `refresh_memory()` every `poll_interval_ms` milliseconds, then writes
    /// the results to the provided `DataVault` and to the internal atomics.
    fn on_init(&mut self, vault: Arc<DataVault>) -> Result<(), PackageError> {
        self.running.store(true, Ordering::SeqCst);

        // Clone Arcs for the background thread.
        let running = Arc::clone(&self.running);
        let poll_ms = Arc::clone(&self.poll_interval_ms);
        let cpu_bits = Arc::clone(&self.cpu_usage_bits);
        let mem_bits = Arc::clone(&self.mem_mb_bits);

        let handle = thread::Builder::new()
            .name("sniffer-perf-monitor".into())
            .spawn(move || {
                let mut sys = sysinfo::System::new();

                // Warm-up: first refresh establishes the CPU baseline.
                sys.refresh_cpu_usage();
                thread::sleep(Duration::from_millis(200));

                while running.load(Ordering::Relaxed) {
                    sys.refresh_cpu_usage();
                    sys.refresh_memory();

                    let cpu = sys.global_cpu_usage();
                    // sysinfo returns bytes; convert to MiB.
                    let mem_mb = sys.used_memory() as f32 / (1024.0 * 1024.0);

                    // Update lock-free atomics for instant reads by query_service.
                    cpu_bits.store(cpu.to_bits(), Ordering::Relaxed);
                    mem_bits.store(mem_mb.to_bits(), Ordering::Relaxed);

                    // Push to DataVault so the JS layer can poll via host_vault_get.
                    // Keys under "pkg.*" are unrestricted (not "system.*" or "plugin.*").
                    let _ = vault.set(
                        "pkg.perf.cpu_usage",
                        format!("{cpu:.1}"),
                        "com.sniffer.perf",
                    );
                    let _ = vault.set(
                        "pkg.perf.mem_mb",
                        format!("{mem_mb:.0}"),
                        "com.sniffer.perf",
                    );

                    let interval = poll_ms.load(Ordering::Relaxed);
                    thread::sleep(Duration::from_millis(interval));
                }
            })
            .map_err(|e| PackageError::Init(e.to_string()))?;

        if let Ok(mut guard) = self.thread_handle.lock() {
            *guard = Some(handle);
        }

        Ok(())
    }

    /// Tracks FPS on the main thread using an exponential moving average.
    ///
    /// `dt_secs` is the frame delta; instant FPS = `1 / dt_secs`.
    /// α = 0.1 gives a smooth readout that responds to sustained changes
    /// within ~10 frames.
    fn on_update(&self, _vault: &DataVault, dt_secs: f32) {
        if dt_secs <= f32::EPSILON {
            return;
        }
        let instant_fps = 1.0 / dt_secs;
        let prev_fps = f32::from_bits(self.fps_ema_bits.load(Ordering::Relaxed));

        let new_fps = if prev_fps < f32::EPSILON {
            // Cold-start: seed with the first observed value.
            instant_fps
        } else {
            FPS_EMA_ALPHA * instant_fps + (1.0 - FPS_EMA_ALPHA) * prev_fps
        };

        self.fps_ema_bits
            .store(new_fps.to_bits(), Ordering::Relaxed);
    }

    /// Adjusts the background polling interval to reduce CPU / battery use.
    fn on_memory_trim(&self, level: MemoryTrimLevel) {
        let interval_ms = match level {
            MemoryTrimLevel::Moderate => POLL_INTERVAL_MODERATE_MS,
            MemoryTrimLevel::Critical => POLL_INTERVAL_CRITICAL_MS,
            MemoryTrimLevel::Emergency => POLL_INTERVAL_EMERGENCY_MS,
        };
        self.poll_interval_ms.store(interval_ms, Ordering::Relaxed);
    }

    /// Signals the background thread to stop and waits for it to exit.
    fn on_unload(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Ok(mut guard) = self.thread_handle.lock() {
            if let Some(handle) = guard.take() {
                // Ignore join errors (thread already exited or panicked).
                let _ = handle.join();
            }
        }
    }

    /// Handles bridge queries from the plugin layer.
    ///
    /// All responses are JSON strings.  Reading from atomics is lock-free
    /// and never blocks the main/render thread.
    fn query_service(&self, method: &str, payload: &str) -> Result<String, PackageError> {
        match method {
            "getCpuUsage" => {
                let cpu = self.read_cpu();
                Ok(format!(r#"{{"ok":true,"cpu_usage":{cpu:.1}}}"#))
            }

            "getMemoryUsage" => {
                let mem = self.read_mem_mb();
                Ok(format!(r#"{{"ok":true,"mem_mb":{mem:.0}}}"#))
            }

            "getAllMetrics" => {
                let cpu = self.read_cpu();
                let mem = self.read_mem_mb();
                let fps = self.read_fps();
                Ok(format!(
                    r#"{{"ok":true,"cpu_usage":{cpu:.1},"mem_mb":{mem:.0},"fps":{fps:.1}}}"#,
                ))
            }

            "setPollingInterval" => {
                let val: serde_json::Value = serde_json::from_str(payload)
                    .map_err(|e| PackageError::InvalidDescriptor(e.to_string()))?;

                let requested = val["interval_ms"].as_u64().ok_or_else(|| {
                    PackageError::InvalidDescriptor(
                        "missing or invalid 'interval_ms' field (expected u64)".into(),
                    )
                })?;

                let clamped = requested.clamp(POLL_INTERVAL_MIN_MS, POLL_INTERVAL_MAX_MS);
                self.poll_interval_ms.store(clamped, Ordering::Relaxed);

                Ok(format!(r#"{{"ok":true,"interval_ms":{clamped}}}"#))
            }

            _ => Err(PackageError::UnsupportedMethod(method.into())),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn make_vault() -> Arc<DataVault> {
        Arc::new(DataVault::default())
    }

    // ── query_service (no thread needed) ────────────────────────────────────

    #[test]
    fn query_get_cpu_usage_returns_valid_json() {
        let pkg = PerfMonitorPackage::new();
        // Atomics default to 0.0; response must still be valid JSON.
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

        // Valid value
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

        // Below minimum → clamped to POLL_INTERVAL_MIN_MS
        let _ = pkg.query_service("setPollingInterval", r#"{"interval_ms":5}"#);
        assert_eq!(
            pkg.poll_interval_ms.load(Ordering::Relaxed),
            POLL_INTERVAL_MIN_MS
        );

        // Above maximum → clamped to POLL_INTERVAL_MAX_MS
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

    // ── MemoryTrimLevel → polling interval mapping ───────────────────────

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

    // ── FPS EMA (main-thread tracking) ──────────────────────────────────

    #[test]
    fn fps_ema_cold_start_seeds_with_first_sample() {
        let pkg = PerfMonitorPackage::new();
        let vault = make_vault();

        // First frame: 60 fps (dt = 1/60)
        pkg.on_update(&vault, 1.0 / 60.0);
        let fps = pkg.read_fps();
        assert!(
            (fps - 60.0).abs() < 1.0,
            "cold-start FPS should seed to ≈60, got {fps}"
        );
    }

    #[test]
    fn fps_ema_smooths_toward_new_rate() {
        let pkg = PerfMonitorPackage::new();
        let vault = make_vault();

        // Warm up at 60 fps.
        for _ in 0..50 {
            pkg.on_update(&vault, 1.0 / 60.0);
        }
        let fps_before = pkg.read_fps();
        assert!((fps_before - 60.0).abs() < 1.0, "should be near 60 fps");

        // Sudden drop to 30 fps.
        for _ in 0..50 {
            pkg.on_update(&vault, 1.0 / 30.0);
        }
        let fps_after = pkg.read_fps();
        // EMA converges slowly; after 50 frames at α=0.1, it should have moved
        // substantially toward 30.
        assert!(
            fps_after < 55.0,
            "EMA should move toward 30 fps, got {fps_after}"
        );
    }

    #[test]
    fn fps_ema_ignores_zero_dt() {
        let pkg = PerfMonitorPackage::new();
        let vault = make_vault();

        // Seed a value.
        pkg.on_update(&vault, 1.0 / 60.0);
        let before = pkg.read_fps();

        // Zero dt must be ignored (division by zero guard).
        pkg.on_update(&vault, 0.0);
        let after = pkg.read_fps();

        assert_eq!(
            before.to_bits(),
            after.to_bits(),
            "zero dt must not change the EMA"
        );
    }

    // ── Lifecycle (init → on_update → unload) ───────────────────────────

    #[test]
    fn init_and_unload_without_panic() {
        let vault = make_vault();
        let mut pkg = PerfMonitorPackage::new();

        pkg.on_init(Arc::clone(&vault))
            .expect("on_init must succeed");
        // Give the background thread a moment to start.
        std::thread::sleep(Duration::from_millis(300));

        // on_update on main thread (simulating a few frames)
        for _ in 0..5 {
            pkg.on_update(&vault, 1.0 / 60.0);
        }

        // Graceful shutdown — must not block indefinitely.
        pkg.on_unload();
    }
}
