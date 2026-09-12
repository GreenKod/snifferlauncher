use rquickjs::{Context, Runtime};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Default per-plugin maximum JS heap usage (8 MiB).
pub const DEFAULT_JS_MEMORY_LIMIT: usize = 8 * 1024 * 1024;
/// Hard floor for JS heap usage (2 MiB).
pub const MIN_JS_MEMORY_LIMIT: usize = 2 * 1024 * 1024;
/// Hard ceiling for JS heap usage (64 MiB).
pub const MAX_JS_MEMORY_LIMIT: usize = 64 * 1024 * 1024;

/// Maximum time allowed for a single JS tick before interrupt fires (ms).
pub(crate) const JS_TICK_DEADLINE_MS: u64 = 500;

/// Sentinel value meaning "no active deadline" — interrupt is disabled.
pub(crate) const DEADLINE_NONE: u64 = u64::MAX;

/// A QuickJS runtime + context pair with a hard memory cap and
/// zero-thread-spawn deadline enforcement via an `AtomicU64` timestamp.
pub struct PluginEngine {
    pub runtime: Runtime,
    pub context: Context,
    /// UNIX epoch milliseconds of the current deadline.
    /// Set to `DEADLINE_NONE` when idle (interrupt disabled).
    /// The interrupt handler reads this on every JS step — no extra thread needed.
    pub deadline_ms: Arc<AtomicU64>,
}

#[inline]
pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as u64
}

/// RAII guard that arms the interrupt watchdog upon creation and disarms it upon drop.
pub struct DeadlineGuard<'a>(&'a AtomicU64);

impl<'a> DeadlineGuard<'a> {
    /// Arms the watchdog for the default `JS_TICK_DEADLINE_MS` (500 ms).
    #[inline]
    #[must_use]
    pub fn arm(deadline_ms: &'a AtomicU64) -> Self {
        deadline_ms.store(now_ms() + JS_TICK_DEADLINE_MS, Ordering::Relaxed);
        Self(deadline_ms)
    }

    /// Arms the watchdog for a custom deadline duration.
    #[inline]
    #[must_use]
    pub fn arm_with_timeout(deadline_ms: &'a AtomicU64, timeout_ms: u64) -> Self {
        deadline_ms.store(now_ms() + timeout_ms, Ordering::Relaxed);
        Self(deadline_ms)
    }
}

impl Drop for DeadlineGuard<'_> {
    #[inline]
    fn drop(&mut self) {
        self.0.store(DEADLINE_NONE, Ordering::Relaxed);
    }
}

/// Create a sandboxed QuickJS engine with a memory limit and atomic deadline interrupts.
///
/// # Errors
///
/// Returns an error string if the QuickJS runtime or context cannot be created.
pub fn create_engine(memory_limit_bytes: Option<usize>) -> Result<PluginEngine, String> {
    let limit = memory_limit_bytes
        .unwrap_or(DEFAULT_JS_MEMORY_LIMIT)
        .clamp(MIN_JS_MEMORY_LIMIT, MAX_JS_MEMORY_LIMIT);

    let runtime = Runtime::new().map_err(|e| format!("QuickJS runtime error: {e}"))?;

    // Hard memory cap — prevents runaway allocation from crashing the launcher.
    runtime.set_memory_limit(limit);

    // Interrupt handler called after every JS instruction.
    // Cost: one atomic load + one u64 comparison. No thread spawn per call.
    let deadline_ms = Arc::new(AtomicU64::new(DEADLINE_NONE));
    let dl = Arc::clone(&deadline_ms);
    runtime.set_interrupt_handler(Some(Box::new(move || {
        let dl_val = dl.load(Ordering::Relaxed);
        dl_val != DEADLINE_NONE && now_ms() >= dl_val
    })));

    let context = Context::full(&runtime).map_err(|e| format!("QuickJS context error: {e}"))?;

    Ok(PluginEngine {
        runtime,
        context,
        deadline_ms,
    })
}

impl PluginEngine {
    /// Arm the deadline timer using an RAII guard.
    #[inline]
    pub fn guard(&self) -> DeadlineGuard<'_> {
        DeadlineGuard::arm(&self.deadline_ms)
    }

    /// Arm the deadline timer before starting a JS tick.
    #[inline]
    pub fn arm_deadline(&self) {
        self.deadline_ms
            .store(now_ms() + JS_TICK_DEADLINE_MS, Ordering::Relaxed);
    }

    /// Disarm the deadline timer after a JS tick completes successfully.
    #[inline]
    pub fn disarm_deadline(&self) {
        self.deadline_ms.store(DEADLINE_NONE, Ordering::Relaxed);
    }
}
