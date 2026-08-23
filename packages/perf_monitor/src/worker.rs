use sniffer_core::vault::DataVault;
use sniffer_pkg::error::PackageError;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
};
use std::thread;
use std::time::Duration;

pub fn spawn_worker_thread(
    running: Arc<AtomicBool>,
    poll_ms: Arc<AtomicU64>,
    cpu_bits: Arc<AtomicU32>,
    mem_bits: Arc<AtomicU32>,
    vault: Arc<DataVault>,
) -> Result<thread::JoinHandle<()>, PackageError> {
    thread::Builder::new()
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
                let mem_mb = sys.used_memory() as f32 / (1024.0 * 1024.0);

                cpu_bits.store(cpu.to_bits(), Ordering::Relaxed);
                mem_bits.store(mem_mb.to_bits(), Ordering::Relaxed);

                let _ = vault.set(
                    "pkg.perf.cpu_usage",
                    &format!("{cpu:.1}"),
                    "com.sniffer.perf",
                );
                let _ = vault.set(
                    "pkg.perf.mem_mb",
                    &format!("{mem_mb:.0}"),
                    "com.sniffer.perf",
                );

                let interval = poll_ms.load(Ordering::Relaxed);
                thread::sleep(Duration::from_millis(interval));
            }
        })
        .map_err(|e| PackageError::Init(e.to_string()))
}
