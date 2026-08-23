use super::*;
use crate::error::PackageError;
use crate::package::{LauncherPackage, MemoryTrimLevel, PackageKind, PackageMeta};
use sniffer_core::vault::DataVault;
use std::sync::Arc;

struct CounterService {
    meta: PackageMeta,
    pub ticks: std::sync::atomic::AtomicU32,
}

impl CounterService {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            meta: PackageMeta {
                id: "com.test.counter",
                version: (0, 1, 0),
                kind: PackageKind::Service,
            },
            ticks: std::sync::atomic::AtomicU32::new(0),
        })
    }
}

impl LauncherPackage for CounterService {
    fn meta(&self) -> &PackageMeta {
        &self.meta
    }
    fn on_init(&mut self, _vault: Arc<DataVault>) -> Result<(), PackageError> {
        Ok(())
    }
    fn on_update(&self, _vault: &DataVault, _dt: f32) {
        self.ticks
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    fn on_memory_trim(&self, _level: MemoryTrimLevel) {}
    fn on_unload(&mut self) {}
}

struct PanicService {
    meta: PackageMeta,
}

impl PanicService {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            meta: PackageMeta {
                id: "com.test.panicker",
                version: (0, 1, 0),
                kind: PackageKind::Service,
            },
        })
    }
}

impl LauncherPackage for PanicService {
    fn meta(&self) -> &PackageMeta {
        &self.meta
    }
    fn on_init(&mut self, _vault: Arc<DataVault>) -> Result<(), PackageError> {
        Ok(())
    }
    fn on_update(&self, _vault: &DataVault, _dt: f32) {
        panic!("intentional panic for testing");
    }
    fn on_memory_trim(&self, _level: MemoryTrimLevel) {}
    fn on_unload(&mut self) {}
}

fn make_registry() -> PackageRegistry {
    PackageRegistry::new(Arc::new(DataVault::default()))
}

#[test]
fn service_tick_increments_counter() {
    let mut reg = make_registry();
    let svc = CounterService::new();
    reg.register_service(svc.clone());

    reg.tick_all(0.016);
    reg.tick_all(0.016);

    assert_eq!(
        svc.ticks.load(std::sync::atomic::Ordering::Relaxed),
        2,
        "on_update must be called once per tick_all"
    );
}

#[test]
fn panic_in_on_update_deactivates_package() {
    let mut reg = make_registry();
    reg.register_service(PanicService::new());
    assert_eq!(reg.faulted_ids().len(), 0);

    // First tick triggers the panic; catch_unwind isolates it.
    reg.tick_all(0.016);

    assert_eq!(
        reg.faulted_ids(),
        vec!["com.test.panicker"],
        "panicking package must be marked faulted"
    );

    // Subsequent tick must not call on_update on the faulted package.
    reg.tick_all(0.016);
}

#[test]
fn clear_faulted_re_enables_package() {
    let mut reg = make_registry();
    reg.register_service(PanicService::new());
    reg.tick_all(0.016);
    assert!(!reg.faulted_ids().is_empty());

    let found = reg.clear_faulted("com.test.panicker");
    assert!(found);
    assert!(reg.faulted_ids().is_empty());
}

#[test]
fn query_service_returns_not_found_for_unknown_id() {
    let reg = make_registry();
    let err = reg.query_service("com.missing", "ping", "{}").unwrap_err();
    assert!(
        matches!(err, PackageError::NotFound(_)),
        "must return NotFound for unregistered package"
    );
}

#[test]
fn service_count_reflects_registrations() {
    let mut reg = make_registry();
    assert_eq!(reg.service_count(), 0);
    reg.register_service(CounterService::new());
    assert_eq!(reg.service_count(), 1);
}

#[test]
fn unload_all_clears_packages() {
    let mut reg = make_registry();
    reg.register_service(CounterService::new());
    reg.unload_all();
    assert_eq!(reg.service_count(), 0);
}
