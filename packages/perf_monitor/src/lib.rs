//! `pkg_perfmon` — System Performance Monitor Package.
//!
//! Independent package crate providing real-time CPU, RAM, and FPS monitoring.
//! Implements [`LauncherPackage`] and exports C-ABI entry points for dynamic loading.

pub mod constants;
pub mod query;
pub mod worker;

#[cfg(test)]
mod tests;

use constants::{FPS_EMA_ALPHA, POLL_INTERVAL_DEFAULT_MS, trim_level_to_interval};
use sniffer_core::vault::DataVault;
use sniffer_pkg::error::PackageError;
use sniffer_pkg::package::{LauncherPackage, MemoryTrimLevel, PackageKind, PackageMeta};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
};
use std::thread;

/// Background service that collects CPU, RAM, and FPS metrics.
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
    fps_ema_bits: Arc<AtomicU32>,
    /// Background thread handle. Taken and joined in `on_unload`.
    thread_handle: Mutex<Option<thread::JoinHandle<()>>>,
}

impl PerfMonitorPackage {
    /// Create a new, uninitialised `PerfMonitorPackage`.
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

impl LauncherPackage for PerfMonitorPackage {
    fn meta(&self) -> &PackageMeta {
        &self.meta
    }

    fn on_init(&mut self, vault: Arc<DataVault>) -> Result<(), PackageError> {
        self.running.store(true, Ordering::SeqCst);

        let handle = worker::spawn_worker_thread(
            Arc::clone(&self.running),
            Arc::clone(&self.poll_interval_ms),
            Arc::clone(&self.cpu_usage_bits),
            Arc::clone(&self.mem_mb_bits),
            vault,
        )?;

        if let Ok(mut guard) = self.thread_handle.lock() {
            *guard = Some(handle);
        }

        Ok(())
    }

    fn on_update(&self, _vault: &DataVault, dt_secs: f32) {
        if dt_secs <= f32::EPSILON {
            return;
        }
        let instant_fps = 1.0 / dt_secs;
        let prev_fps = f32::from_bits(self.fps_ema_bits.load(Ordering::Relaxed));

        let new_fps = if prev_fps < f32::EPSILON {
            instant_fps
        } else {
            FPS_EMA_ALPHA * instant_fps + (1.0 - FPS_EMA_ALPHA) * prev_fps
        };

        self.fps_ema_bits
            .store(new_fps.to_bits(), Ordering::Relaxed);
    }

    fn on_memory_trim(&self, level: MemoryTrimLevel) {
        let interval_ms = trim_level_to_interval(level);
        self.poll_interval_ms.store(interval_ms, Ordering::Relaxed);
    }

    fn on_unload(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Ok(mut guard) = self.thread_handle.lock() {
            if let Some(handle) = guard.take() {
                let _ = handle.join();
            }
        }
    }

    fn query_service(&self, method: &str, payload: &str) -> Result<String, PackageError> {
        query::handle_query(
            method,
            payload,
            self.read_cpu(),
            self.read_mem_mb(),
            self.read_fps(),
            &self.poll_interval_ms,
        )
    }
}

/// C-ABI entry point for dynamic loading (`libloading` / dlopen).
///
/// # Safety
/// The caller must ensure the returned pointer is wrapped in an `Arc<dyn LauncherPackage>`
/// using [`Arc::from_raw`].
#[unsafe(no_mangle)]
pub extern "C" fn sniffer_pkg_create() -> *mut Box<dyn LauncherPackage> {
    let pkg: Box<dyn LauncherPackage> = Box::new(PerfMonitorPackage::new());
    Box::into_raw(Box::new(pkg))
}

/// Fallback C-ABI entry point matching `pkg_create`.
///
/// # Safety
/// Same as `sniffer_pkg_create`.
#[unsafe(no_mangle)]
pub extern "C" fn pkg_create() -> *mut Box<dyn LauncherPackage> {
    sniffer_pkg_create()
}
