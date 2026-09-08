use rquickjs::{Context, Runtime};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Per-plugin maximum JS heap usage (8 MiB).
const JS_MEMORY_LIMIT: usize = 8 * 1024 * 1024;

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

/// Create a sandboxed QuickJS engine with a memory limit and atomic deadline interrupts.
///
/// # Errors
///
/// Returns an error string if the QuickJS runtime or context cannot be created.
pub fn create_engine() -> Result<PluginEngine, String> {
    let runtime = Runtime::new().map_err(|e| format!("QuickJS runtime error: {e}"))?;

    // Hard memory cap — prevents runaway allocation from crashing the launcher.
    runtime.set_memory_limit(JS_MEMORY_LIMIT);

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
    /// Arm the deadline timer before starting a JS tick.
    ///
    /// Stores `now + JS_TICK_DEADLINE_MS` atomically. No thread is spawned.
    #[inline]
    pub fn arm_deadline(&self) {
        self.deadline_ms
            .store(now_ms() + JS_TICK_DEADLINE_MS, Ordering::Relaxed);
    }

    /// Disarm the deadline timer after a JS tick completes successfully.
    ///
    /// Stores the sentinel `DEADLINE_NONE` so the handler never fires when idle.
    #[inline]
    pub fn disarm_deadline(&self) {
        self.deadline_ms.store(DEADLINE_NONE, Ordering::Relaxed);
    }
}
