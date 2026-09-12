#![cfg(not(target_os = "android"))]

use sniffer_plugin::js::engine::{DeadlineGuard, create_engine};
use std::time::Instant;

#[test]
fn test_watchdog_interrupts_infinite_loop() {
    let engine = create_engine(Some(8 * 1024 * 1024)).expect("Engine creation failed");
    let sniffer_plugin::js::engine::PluginEngine {
        runtime: _runtime,
        context,
        deadline_ms,
    } = engine;

    let start = Instant::now();
    let res: Result<(), String> = context.with(|ctx| {
        // Arm watchdog with a 50ms deadline
        let _guard = DeadlineGuard::arm_with_timeout(&deadline_ms, 50);
        ctx.eval::<(), _>(b"let count = 0; while (true) { count++; }")
            .map_err(|e| e.to_string())
    });

    let elapsed = start.elapsed();
    assert!(
        res.is_err(),
        "Infinite loop should be interrupted by watchdog"
    );
    assert!(
        elapsed.as_millis() < 500,
        "Watchdog should have interrupted loop in under 500ms, took {elapsed:?}"
    );

    // Verify context is still functional for normal evaluation after watchdog disarms
    let subsequent_res: Result<i32, _> = context.with(|ctx| ctx.eval(b"40 + 2"));
    assert_eq!(subsequent_res.unwrap(), 42);
}

#[test]
fn test_memory_limit_prevents_runaway_allocation() {
    // Set 2 MiB limit (floor)
    let engine = create_engine(Some(2 * 1024 * 1024)).expect("Engine creation failed");
    let sniffer_plugin::js::engine::PluginEngine {
        runtime: _runtime,
        context,
        deadline_ms: _deadline_ms,
    } = engine;

    let res: Result<(), String> = context.with(|ctx| {
        // Attempt to allocate ~10MB array in a 2MB sandbox
        ctx.eval::<(), _>(b"const arr = []; for (let i = 0; i < 500000; i++) { arr.push({ id: i, text: 'runaway allocation test' }); }")
            .map_err(|e| e.to_string())
    });

    assert!(
        res.is_err(),
        "Allocation exceeding memory cap should be rejected with OOM"
    );
}
