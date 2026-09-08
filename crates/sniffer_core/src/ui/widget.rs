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

/// Returns true if an element ID represents a purely structural layout container
/// (e.g. root, wrapper, layer, track, grid, spacer) rather than an interactive target/button.
#[must_use]
pub fn is_structural_layout_id(id: &str) -> bool {
    let lower = id.to_ascii_lowercase();
    lower == "root"
        || lower.ends_with("_wrapper")
        || lower.ends_with("_layer")
        || lower.ends_with("_root")
        || lower.ends_with("_track")
        || lower.ends_with("_grid")
        || lower.ends_with("_header")
        || lower.ends_with("_port")
        || lower.ends_with("_area")
        || lower.ends_with("_spacer")
        || lower.ends_with("_empty")
        || lower.ends_with("_scroll")
        || lower.starts_with("page_grid_")
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

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    pub fn test_fnv1a_basic() {
        assert_eq!(fnv1a(b"test"), 18_007_334_074_686_647_077_u64);
    }

    #[test]
    pub fn test_fnv1a_empty() {
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
    }

    #[test]
    pub fn test_fnv1a_collision_resistance() {
        let h1 = fnv1a(b"button_1");
        let h2 = fnv1a(b"button_2");
        assert_ne!(h1, h2);
    }

    #[test]
    pub fn test_fnv1a_uniqueness() {
        let mut hashes = std::collections::HashSet::new();
        for i in 0..1000 {
            let s = format!("widget_{i}");
            assert!(hashes.insert(fnv1a(s.as_bytes())));
        }
    }

    #[test]
    pub fn test_is_structural_layout_id() {
        assert!(is_structural_layout_id("root"));
        assert!(is_structural_layout_id("home_screen_layer"));
        assert!(is_structural_layout_id("drawer_grid_wrapper"));
        assert!(is_structural_layout_id("app_drawer_root"));
        assert!(is_structural_layout_id("app_grid_track"));
        assert!(is_structural_layout_id("page_grid_0"));
        assert!(is_structural_layout_id("actions_grid"));
        assert!(is_structural_layout_id("drawer_header"));

        assert!(!is_structural_layout_id("app_btn_1"));
        assert!(!is_structural_layout_id("drawer_card_0"));
        assert!(!is_structural_layout_id("dock_app_btn_com_example"));
        assert!(!is_structural_layout_id("drawer_close_btn"));
        assert!(!is_structural_layout_id("drawer_search_container"));
    }
}
