/// Unique identifier for every UI widget.
/// Computed via FNV-1a hash of a `&[u8]` literal at compile time — zero runtime cost.
pub type WidgetId = u64;

/// Compute a `WidgetId` (u64) from a byte-string literal at runtime or compile time.
#[must_use]
pub const fn fnv1a(s: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325_u64;
    let mut i = 0usize;
    while i < s.len() {
        hash ^= s[i] as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01B3_u64);
        i += 1;
    }
    hash
}

/// Compute a `WidgetId` (u64) from a byte-string literal at compile time.
///
/// Uses the FNV-1a hash algorithm; collisions are astronomically unlikely for
/// typical widget name sets.
///
/// # Example
/// ```rust,ignore
/// const MY_BUTTON: u64 = snifferlauncher::wid!(b"my-button");
/// ```
#[macro_export]
macro_rules! wid {
    ($s:literal) => {{ $crate::ui::widget::fnv1a($s) }};
}
