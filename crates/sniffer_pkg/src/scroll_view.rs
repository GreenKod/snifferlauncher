//! `ScrollViewPackage` — Native GPU scroll widget.
//!
//! Implements [`WidgetPackage`] to provide hardware-accelerated scrolling with:
//! - **Momentum scrolling** — inertia carries the content after a fling.
//! - **Rubber-band (elastic) boundaries** — content stretches past the edge
//!   and snaps back with a spring-restoring force.
//! - **Snap-to-page** — horizontal or vertical pager mode; content snaps to
//!   the nearest page boundary when velocity falls below the snap threshold.
//! - **Scrollbar indicator** — a translucent thumb drawn on top of the content.
//! - **Page dots** — optional horizontal dot indicator for pager mode.
//!
//! ## Configuration via `apply_descriptor`
//!
//! The JavaScript bridge calls `host_extend_widget` which routes to this
//! method.  The JSON descriptor fields are all optional:
//!
//! ```json
//! {
//!   "snap_x": 400.0,
//!   "snap_y": null,
//!   "rubber_band": 0.3,
//!   "page_count": 5,
//!   "max_scroll_x": 2000.0,
//!   "max_scroll_y": 0.0,
//!   "momentum_damping": 0.015,
//!   "on_snap": "onPageChanged"
//! }
//! ```
//!
//! ## External scroll input
//!
//! Call [`ScrollViewPackage::apply_scroll_delta`] from the input layer when
//! a drag/fling gesture is recognised.  This is not part of the
//! [`WidgetPackage`] trait because it is input-system-specific.

use crate::error::PackageError;
use crate::package::{MemoryTrimLevel, PackageKind, PackageMeta, WidgetPackage};
use sniffer_core::{
    math::{Rect, Size},
    render_api::Renderer,
    vault::DataVault,
};
use std::sync::{Arc, RwLock};

// ---------------------------------------------------------------------------
// Physics constants
// ---------------------------------------------------------------------------

/// Velocity threshold below which snap-to-page activates (px/s).
const SNAP_VELOCITY_THRESHOLD: f32 = 50.0;

/// Spring stiffness for snap / rubber-band return animation (px/s² per px).
const SPRING_STIFFNESS: f32 = 12.0;

/// Damping applied to rubber-band overshoot per second (multiplied by
/// `rubber_band` config value).
const RUBBER_BAND_RESTORING: f32 = 8.0;

/// Scrollbar thumb minimum height as a fraction of the viewport height.
const SCROLLBAR_MIN_THUMB_RATIO: f32 = 0.05;

/// Scrollbar thumb width in physical pixels.
const SCROLLBAR_WIDTH: f32 = 3.0;

/// Page dot diameter in physical pixels.
const PAGE_DOT_SIZE: f32 = 6.0;

/// Spacing between the centre of adjacent page dots in physical pixels.
const PAGE_DOT_SPACING: f32 = 10.0;

/// Margin from the bottom edge of the scroll area to the page dot row (px).
const PAGE_DOT_MARGIN_BOTTOM: f32 = 14.0;

// ---------------------------------------------------------------------------
// ScrollState
// ---------------------------------------------------------------------------

/// Mutable physics and configuration state for a single scroll view.
#[derive(Clone, Debug)]
struct ScrollState {
    // ── Position ──────────────────────────────────────────────────────────
    /// Horizontal scroll offset in physical pixels.
    scroll_x: f32,
    /// Vertical scroll offset in physical pixels.
    scroll_y: f32,

    // ── Velocity (px/s) ────────────────────────────────────────────────
    velocity_x: f32,
    velocity_y: f32,

    // ── Layout ─────────────────────────────────────────────────────────
    /// Maximum reachable horizontal scroll offset.  Set via the descriptor.
    max_scroll_x: f32,
    /// Maximum reachable vertical scroll offset.  Set via the descriptor.
    max_scroll_y: f32,

    // ── Snap ───────────────────────────────────────────────────────────
    /// Snap interval in px for the X axis (pager width).  `None` = free scroll.
    snap_x: Option<f32>,
    /// Snap interval in px for the Y axis.  `None` = free scroll.
    snap_y: Option<f32>,
    /// Number of pages (used for page-dot rendering).
    page_count: u32,
    /// Current page index after the last completed snap.
    current_page: i32,

    // ── Physics config ─────────────────────────────────────────────────
    /// Elasticity at boundaries. `0.0` = rigid clamp, `1.0` = fully elastic.
    rubber_band: f32,
    /// Per-frame velocity decay: `velocity *= (1 - damping)^(dt * 60)`.
    /// Typical value: 0.015 → ~60 fps half-life ≈ 46 frames.
    momentum_damping: f32,

    // ── Callback ───────────────────────────────────────────────────────
    /// JS function name invoked when `on_snap` occurs (via DataVault signal).
    on_snap_callback: Option<String>,
}

impl Default for ScrollState {
    fn default() -> Self {
        Self {
            scroll_x: 0.0,
            scroll_y: 0.0,
            velocity_x: 0.0,
            velocity_y: 0.0,
            max_scroll_x: 0.0,
            max_scroll_y: 0.0,
            snap_x: None,
            snap_y: None,
            page_count: 0,
            current_page: 0,
            rubber_band: 0.3,
            momentum_damping: 0.015,
            on_snap_callback: None,
        }
    }
}

// ---------------------------------------------------------------------------
// ScrollViewPackage
// ---------------------------------------------------------------------------

/// Native GPU widget that hosts a scrollable content area.
///
/// Register as a widget package, then configure via `apply_descriptor` or
/// the JS bridge.
pub struct ScrollViewPackage {
    meta: PackageMeta,
    /// All mutable scroll state behind an `RwLock` so that `on_update` and
    /// `on_render` (both `&self`) can share it safely.
    state: RwLock<ScrollState>,
}

impl ScrollViewPackage {
    /// Create a new scroll view widget with default physics settings.
    #[must_use]
    pub fn new() -> Self {
        Self {
            meta: PackageMeta {
                id: "com.sniffer.scroll_view",
                version: (1, 0, 0),
                kind: PackageKind::Widget,
            },
            state: RwLock::new(ScrollState::default()),
        }
    }

    /// Apply an external scroll delta (from the gesture/input layer).
    ///
    /// `dx` / `dy` are pixel distances for this gesture event.
    /// `velocity_x` / `velocity_y` are the fling velocity in px/s
    /// (set to zero for non-fling drag events).
    pub fn apply_scroll_delta(&self, dx: f32, dy: f32, velocity_x: f32, velocity_y: f32) {
        let Ok(mut state) = self.state.write() else {
            return;
        };
        state.scroll_x += dx;
        state.scroll_y += dy;
        // A non-zero velocity overrides; a zero velocity from a drag does not
        // reset momentum that was already in flight.
        if velocity_x.abs() > f32::EPSILON || velocity_y.abs() > f32::EPSILON {
            state.velocity_x = velocity_x;
            state.velocity_y = velocity_y;
        }
    }

    /// Read the current scroll offsets without holding the write lock.
    pub fn scroll_position(&self) -> (f32, f32) {
        self.state
            .read()
            .map_or((0.0, 0.0), |s| (s.scroll_x, s.scroll_y))
    }

    /// Read the current active page (after the last snap).
    pub fn current_page(&self) -> i32 {
        self.state.read().map_or(0, |s| s.current_page)
    }
}

impl Default for ScrollViewPackage {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// WidgetPackage implementation
// ---------------------------------------------------------------------------

impl WidgetPackage for ScrollViewPackage {
    fn meta(&self) -> &PackageMeta {
        &self.meta
    }

    fn on_init(&mut self, _vault: Arc<DataVault>) -> Result<(), PackageError> {
        Ok(())
    }

    /// Advances the scroll physics simulation by `dt_secs`.
    ///
    /// Steps performed each frame:
    /// 1. Apply momentum damping.
    /// 2. Integrate velocity → position.
    /// 3. Apply rubber-band restoring force at boundaries.
    /// 4. Snap to the nearest page when velocity is below threshold.
    fn on_update(&self, _vault: &DataVault, dt_secs: f32) {
        if dt_secs <= f32::EPSILON {
            return;
        }

        let Ok(mut st) = self.state.write() else {
            return;
        };

        // ── 1. Momentum damping ────────────────────────────────────────────
        // Exponential decay: v(t) = v₀ · (1 - damping)^(t·60)
        let decay = (1.0 - st.momentum_damping).powf(dt_secs * 60.0);
        st.velocity_x *= decay;
        st.velocity_y *= decay;

        // ── 2. Integrate position ─────────────────────────────────────────
        st.scroll_x += st.velocity_x * dt_secs;
        st.scroll_y += st.velocity_y * dt_secs;

        // ── 3. Boundary handling ──────────────────────────────────────────
        let rb = st.rubber_band;

        if rb > f32::EPSILON {
            // Elastic boundaries: restoring spring force pushes back.
            if st.scroll_x < 0.0 {
                st.velocity_x += (-st.scroll_x) * rb * RUBBER_BAND_RESTORING * dt_secs;
            } else if st.max_scroll_x > 0.0 && st.scroll_x > st.max_scroll_x {
                st.velocity_x -=
                    (st.scroll_x - st.max_scroll_x) * rb * RUBBER_BAND_RESTORING * dt_secs;
            }

            if st.scroll_y < 0.0 {
                st.velocity_y += (-st.scroll_y) * rb * RUBBER_BAND_RESTORING * dt_secs;
            } else if st.max_scroll_y > 0.0 && st.scroll_y > st.max_scroll_y {
                st.velocity_y -=
                    (st.scroll_y - st.max_scroll_y) * rb * RUBBER_BAND_RESTORING * dt_secs;
            }
        } else {
            // Rigid boundaries: hard clamp.
            st.scroll_x = st.scroll_x.clamp(0.0, st.max_scroll_x.max(0.0));
            st.scroll_y = st.scroll_y.clamp(0.0, st.max_scroll_y.max(0.0));
            if st.scroll_x == 0.0 || st.scroll_x == st.max_scroll_x {
                st.velocity_x = 0.0;
            }
            if st.scroll_y == 0.0 || st.scroll_y == st.max_scroll_y {
                st.velocity_y = 0.0;
            }
        }

        // ── 4. Snap-to-page (X axis) ──────────────────────────────────────
        if let Some(snap_x) = st.snap_x {
            if snap_x > f32::EPSILON && st.velocity_x.abs() < SNAP_VELOCITY_THRESHOLD {
                let max_page = st.page_count.saturating_sub(1) as f32;
                let target_page = (st.scroll_x / snap_x).round().clamp(0.0, max_page);
                let target_x = target_page * snap_x;

                // Spring step toward target.
                let delta = (target_x - st.scroll_x) * SPRING_STIFFNESS * dt_secs;
                st.scroll_x += delta;

                // Update active page only when we are close enough to the target.
                let new_page = target_page as i32;
                if new_page != st.current_page && (st.scroll_x - target_x).abs() < 1.0 {
                    st.current_page = new_page;
                }
            }
        }

        // ── 4b. Snap-to-page (Y axis) ────────────────────────────────────
        if let Some(snap_y) = st.snap_y {
            if snap_y > f32::EPSILON && st.velocity_y.abs() < SNAP_VELOCITY_THRESHOLD {
                let max_page = st.page_count.saturating_sub(1) as f32;
                let target_page = (st.scroll_y / snap_y).round().clamp(0.0, max_page);
                let target_y = target_page * snap_y;
                st.scroll_y += (target_y - st.scroll_y) * SPRING_STIFFNESS * dt_secs;
            }
        }
    }

    /// Renders scroll overlays: clip boundary, scrollbar thumb, and page dots.
    ///
    /// This method sets up the clip region and scroll transform for the
    /// content area, draws the scrollbar indicator and page dots on top,
    /// then tears down the clip/transform stack symmetrically.
    ///
    /// **Content children** are rendered by the main element-tree pass
    /// (which reads the scroll offsets from DataVault or a direct reference
    /// and runs after this pre-render hook).
    fn on_render(&self, renderer: &mut dyn Renderer, layout_rect: Rect, clip_rect: Option<Rect>) {
        let Ok(st) = self.state.read() else {
            return;
        };

        // ── Clip to scroll area boundary ──────────────────────────────────
        let effective_clip = clip_rect.unwrap_or(layout_rect);
        renderer.push_clip_rect(effective_clip, 0.0);

        // ── Scroll content transform ──────────────────────────────────────
        // Children drawn after this call will be offset by the scroll position.
        renderer.push_transform(
            0.0,
            0.0,
            1.0, // scale
            0.0, // rotation
            -st.scroll_x,
            -st.scroll_y,
        );
        // NOTE: the main render pass draws child elements here with the
        //       transform active.

        renderer.pop_transform();

        // ── Scrollbar indicator (vertical) ───────────────────────────────
        if st.max_scroll_y > f32::EPSILON {
            let track_h = layout_rect.height;
            let thumb_ratio = (layout_rect.height / (layout_rect.height + st.max_scroll_y))
                .clamp(SCROLLBAR_MIN_THUMB_RATIO, 1.0);
            let thumb_h = track_h * thumb_ratio;
            let thumb_y = layout_rect.y
                + (st.scroll_y / st.max_scroll_y).clamp(0.0, 1.0) * (track_h - thumb_h);
            let thumb_x = layout_rect.x + layout_rect.width - SCROLLBAR_WIDTH - 1.0;

            renderer.draw_rect(
                Rect::new(thumb_x, thumb_y, SCROLLBAR_WIDTH, thumb_h),
                0x66_FFFFFF, // translucent white
                SCROLLBAR_WIDTH / 2.0,
                0.0,
                None,
            );
        }

        // ── Scrollbar indicator (horizontal) ─────────────────────────────
        if st.max_scroll_x > f32::EPSILON && st.snap_x.is_none() {
            let track_w = layout_rect.width;
            let thumb_ratio = (layout_rect.width / (layout_rect.width + st.max_scroll_x))
                .clamp(SCROLLBAR_MIN_THUMB_RATIO, 1.0);
            let thumb_w = track_w * thumb_ratio;
            let thumb_x = layout_rect.x
                + (st.scroll_x / st.max_scroll_x).clamp(0.0, 1.0) * (track_w - thumb_w);
            let thumb_y = layout_rect.y + layout_rect.height - SCROLLBAR_WIDTH - 1.0;

            renderer.draw_rect(
                Rect::new(thumb_x, thumb_y, thumb_w, SCROLLBAR_WIDTH),
                0x66_FFFFFF,
                SCROLLBAR_WIDTH / 2.0,
                0.0,
                None,
            );
        }

        // ── Page dots (horizontal snap / pager mode) ──────────────────────
        if st.snap_x.is_some() && st.page_count > 1 {
            let count = st.page_count as usize;
            let total_w = count as f32 * PAGE_DOT_SPACING - (PAGE_DOT_SPACING - PAGE_DOT_SIZE);
            let start_x = layout_rect.x + (layout_rect.width - total_w) * 0.5;
            let dot_cy = layout_rect.y + layout_rect.height - PAGE_DOT_MARGIN_BOTTOM;

            for i in 0..count {
                let cx = start_x + i as f32 * PAGE_DOT_SPACING + PAGE_DOT_SIZE * 0.5;
                let is_active = i32::try_from(i).is_ok_and(|idx| idx == st.current_page);
                let color = if is_active { 0xFF_FFFFFF } else { 0x44_FFFFFF };
                renderer.draw_circle(cx, dot_cy, PAGE_DOT_SIZE * 0.5, color);
            }
        }

        // ── Pop clip ─────────────────────────────────────────────────────
        renderer.pop_clip_rect();
    }

    fn on_memory_trim(&self, _level: MemoryTrimLevel) {
        // No GPU resources to release in the base ScrollViewPackage.
        // A future version with texture-cached content could evict here.
    }

    fn on_unload(&mut self) {
        // Nothing to clean up.
    }

    /// Applies a JSON descriptor to update physics and snap configuration.
    ///
    /// All fields are optional; omitted fields retain their current values.
    ///
    /// # Errors
    ///
    /// Returns [`PackageError::InvalidDescriptor`] if the JSON cannot be parsed.
    fn apply_descriptor(&self, descriptor_json: &str) -> Result<(), PackageError> {
        #[derive(serde::Deserialize)]
        struct Descriptor {
            snap_x: Option<f32>,
            snap_y: Option<f32>,
            rubber_band: Option<f32>,
            page_count: Option<u32>,
            max_scroll_x: Option<f32>,
            max_scroll_y: Option<f32>,
            momentum_damping: Option<f32>,
            on_snap: Option<String>,
        }

        let desc: Descriptor = serde_json::from_str(descriptor_json)
            .map_err(|e| PackageError::InvalidDescriptor(e.to_string()))?;

        let Ok(mut st) = self.state.write() else {
            return Err(PackageError::Poisoned("scroll_view.state".into()));
        };

        if let Some(v) = desc.snap_x {
            st.snap_x = if v > f32::EPSILON { Some(v) } else { None };
        }
        if let Some(v) = desc.snap_y {
            st.snap_y = if v > f32::EPSILON { Some(v) } else { None };
        }
        if let Some(v) = desc.rubber_band {
            st.rubber_band = v.clamp(0.0, 1.0);
        }
        if let Some(v) = desc.page_count {
            st.page_count = v;
        }
        if let Some(v) = desc.max_scroll_x {
            st.max_scroll_x = v.max(0.0);
        }
        if let Some(v) = desc.max_scroll_y {
            st.max_scroll_y = v.max(0.0);
        }
        if let Some(v) = desc.momentum_damping {
            // Clamp to (0, 1) — 0 = no damping, 1 = instant stop.
            st.momentum_damping = v.clamp(0.0, 1.0);
        }
        if let Some(cb) = desc.on_snap {
            st.on_snap_callback = if cb.is_empty() { None } else { Some(cb) };
        }

        Ok(())
    }

    fn measure_content(&self, available: Size) -> Size {
        available
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use sniffer_core::style::ObjectFit;
    use std::sync::Arc;

    // ── Minimal mock renderer ────────────────────────────────────────────

    /// No-op renderer that records how many times `draw_circle` and
    /// `draw_rect` were called (for overlay assertion tests).
    #[derive(Default)]
    struct MockRenderer {
        pub rects_drawn: u32,
        pub circles_drawn: u32,
        pub clip_depth: i32,
        pub transform_depth: i32,
    }

    impl Renderer for MockRenderer {
        fn clear(&mut self, _color: u32) {}

        fn draw_rect(
            &mut self,
            _rect: Rect,
            _color: u32,
            _radius: f32,
            _border_width: f32,
            _border_color: Option<u32>,
        ) {
            self.rects_drawn += 1;
        }

        fn draw_rect_gradient(
            &mut self,
            _rect: Rect,
            _color_top: u32,
            _color_bottom: u32,
            _radius: f32,
            _border_width: f32,
            _border_color: Option<u32>,
        ) {
        }

        fn draw_shadow(
            &mut self,
            _rect: Rect,
            _radius: f32,
            _offset_y: f32,
            _spread: f32,
            _color: u32,
        ) {
        }

        fn draw_circle(&mut self, _cx: f32, _cy: f32, _radius: f32, _color: u32) {
            self.circles_drawn += 1;
        }

        fn draw_text(&mut self, _text: &str, _x: f32, _y: f32, _size: f32, _color: u32) {}

        fn begin_frame(&mut self, _width: f32, _height: f32) {}
        fn end_frame(&mut self) {}

        fn set_clip_rect(&mut self, _rect: Rect) {
            self.clip_depth += 1;
        }

        fn clear_clip_rect(&mut self) {
            self.clip_depth -= 1;
        }

        fn push_clip_rect(&mut self, _rect: Rect, _radius: f32) {
            self.clip_depth += 1;
        }

        fn pop_clip_rect(&mut self) {
            self.clip_depth -= 1;
        }

        fn push_transform(
            &mut self,
            _cx: f32,
            _cy: f32,
            _scale: f32,
            _rotate: f32,
            _tx: f32,
            _ty: f32,
        ) {
            self.transform_depth += 1;
        }

        fn pop_transform(&mut self) {
            self.transform_depth -= 1;
        }

        fn set_global_alpha(&mut self, _alpha: f32) {}

        fn load_image(&mut self, _id: &str, _pixels: &[u8], _w: u32, _h: u32) {}

        fn has_image(&self, _id: &str) -> bool {
            false
        }

        fn draw_image(&mut self, _id: &str, _rect: Rect, _radius: f32, _object_fit: ObjectFit) {}

        fn measure_text(&self, _text: &str, _size: f32) -> f32 {
            0.0
        }
    }

    fn make_vault() -> Arc<DataVault> {
        Arc::new(DataVault::default())
    }

    fn layout_rect() -> Rect {
        Rect::new(0.0, 0.0, 400.0, 800.0)
    }

    // ── apply_descriptor ────────────────────────────────────────────────

    #[test]
    fn descriptor_sets_snap_x_and_page_count() {
        let pkg = ScrollViewPackage::new();
        pkg.apply_descriptor(r#"{"snap_x":400.0,"page_count":5,"rubber_band":0.25}"#)
            .expect("valid descriptor must not error");

        let st = pkg.state.read().unwrap();
        assert_eq!(st.snap_x, Some(400.0));
        assert_eq!(st.page_count, 5);
        assert!((st.rubber_band - 0.25).abs() < f32::EPSILON);
    }

    #[test]
    fn descriptor_zero_snap_x_disables_snap() {
        let pkg = ScrollViewPackage::new();
        // First enable snap
        pkg.apply_descriptor(r#"{"snap_x":400.0}"#).unwrap();
        assert!(pkg.state.read().unwrap().snap_x.is_some());

        // Then disable by setting to 0
        pkg.apply_descriptor(r#"{"snap_x":0.0}"#).unwrap();
        assert!(
            pkg.state.read().unwrap().snap_x.is_none(),
            "zero snap_x must disable snapping"
        );
    }

    #[test]
    fn descriptor_partial_update_preserves_other_fields() {
        let pkg = ScrollViewPackage::new();
        pkg.apply_descriptor(r#"{"snap_x":400.0,"page_count":3}"#)
            .unwrap();
        // Only update rubber_band — snap_x and page_count must be preserved.
        pkg.apply_descriptor(r#"{"rubber_band":0.5}"#).unwrap();
        let st = pkg.state.read().unwrap();
        assert_eq!(st.snap_x, Some(400.0), "snap_x must be preserved");
        assert_eq!(st.page_count, 3, "page_count must be preserved");
        assert!((st.rubber_band - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn descriptor_invalid_json_returns_error() {
        let pkg = ScrollViewPackage::new();
        let err = pkg.apply_descriptor("not json").unwrap_err();
        assert!(matches!(err, PackageError::InvalidDescriptor(_)));
    }

    #[test]
    fn descriptor_clamps_rubber_band_to_0_1() {
        let pkg = ScrollViewPackage::new();
        pkg.apply_descriptor(r#"{"rubber_band":5.0}"#).unwrap();
        assert!(
            (pkg.state.read().unwrap().rubber_band - 1.0).abs() < f32::EPSILON,
            "rubber_band above 1.0 must be clamped to 1.0"
        );

        pkg.apply_descriptor(r#"{"rubber_band":-1.0}"#).unwrap();
        assert!(
            pkg.state.read().unwrap().rubber_band.abs() < f32::EPSILON,
            "rubber_band below 0.0 must be clamped to 0.0"
        );
    }

    // ── Physics: momentum damping ────────────────────────────────────────

    #[test]
    fn momentum_decays_to_near_zero_after_many_frames() {
        let pkg = ScrollViewPackage::new();
        let vault = make_vault();

        {
            let mut st = pkg.state.write().unwrap();
            st.velocity_x = 1000.0; // 1000 px/s initial
            st.max_scroll_x = 100_000.0; // large enough not to clamp
        }

        // Simulate ~8.3 seconds at 60 fps = 500 frames
        for _ in 0..500 {
            pkg.on_update(&vault, 1.0 / 60.0);
        }

        let st = pkg.state.read().unwrap();
        assert!(
            st.velocity_x.abs() < 1.0,
            "velocity must decay to <1 px/s after 500 frames, got {}",
            st.velocity_x
        );
    }

    #[test]
    fn scroll_position_integrates_velocity() {
        let pkg = ScrollViewPackage::new();
        let vault = make_vault();

        {
            let mut st = pkg.state.write().unwrap();
            st.velocity_x = 100.0; // 100 px/s
            st.max_scroll_x = 100_000.0; // no boundary effect
            st.momentum_damping = 0.0; // no damping for predictable test
        }

        // 1 frame at dt = 0.1 s → expected Δx ≈ 10 px (damping = 0)
        pkg.on_update(&vault, 0.1);
        let (x, _) = pkg.scroll_position();
        assert!((x - 10.0).abs() < 0.5, "expected scroll_x ≈ 10.0, got {x}");
    }

    // ── Physics: rubber-band ─────────────────────────────────────────────

    #[test]
    fn rubber_band_returns_overshoot_toward_zero() {
        let pkg = ScrollViewPackage::new();
        let vault = make_vault();

        // Start 50 px past the left boundary.
        {
            let mut st = pkg.state.write().unwrap();
            st.scroll_x = -50.0;
            st.velocity_x = 0.0;
            st.rubber_band = 0.5;
            st.momentum_damping = 0.0;
        }

        // Run 60 frames (1 second).
        for _ in 0..60 {
            pkg.on_update(&vault, 1.0 / 60.0);
        }

        let (x, _) = pkg.scroll_position();
        assert!(
            x > -50.0,
            "rubber-band must push scroll_x back toward 0, got {x}"
        );
    }

    #[test]
    fn rigid_boundary_clamps_position() {
        let pkg = ScrollViewPackage::new();
        let vault = make_vault();

        {
            let mut st = pkg.state.write().unwrap();
            st.scroll_x = -200.0; // past left boundary
            st.velocity_x = -500.0;
            st.rubber_band = 0.0; // rigid mode
            st.max_scroll_x = 1000.0;
        }

        pkg.on_update(&vault, 1.0 / 60.0);

        let (x, _) = pkg.scroll_position();
        assert!(
            x >= 0.0,
            "rigid boundary must clamp scroll_x to >= 0, got {x}"
        );
    }

    // ── Physics: snap-to-page ────────────────────────────────────────────

    #[test]
    fn snap_springs_toward_nearest_page() {
        let pkg = ScrollViewPackage::new();
        let vault = make_vault();

        {
            let mut st = pkg.state.write().unwrap();
            st.snap_x = Some(400.0);
            st.page_count = 3;
            st.scroll_x = 210.0; // closer to page 1 (400) than page 0 (0)
            st.velocity_x = 0.0; // below threshold
        }

        // Let the spring run for 60 frames.
        for _ in 0..60 {
            pkg.on_update(&vault, 1.0 / 60.0);
        }

        let (x, _) = pkg.scroll_position();
        assert!(
            (x - 400.0).abs() < 5.0,
            "snap must spring to page 1 (400 px), got {x}"
        );
        assert_eq!(pkg.current_page(), 1, "current_page must be 1 after snap");
    }

    #[test]
    fn no_snap_when_velocity_above_threshold() {
        let pkg = ScrollViewPackage::new();
        let vault = make_vault();

        {
            let mut st = pkg.state.write().unwrap();
            st.snap_x = Some(400.0);
            st.page_count = 3;
            st.scroll_x = 210.0;
            st.velocity_x = 500.0; // above SNAP_VELOCITY_THRESHOLD
            st.max_scroll_x = 800.0;
            st.momentum_damping = 0.0;
        }

        // Single frame — should NOT snap yet (velocity too high).
        pkg.on_update(&vault, 1.0 / 60.0);

        let (x, _) = pkg.scroll_position();
        // Position should move by velocity * dt, not spring toward 400.
        assert!(
            x > 210.0,
            "position must advance with velocity when above snap threshold, got {x}"
        );
    }

    // ── on_render overlays ───────────────────────────────────────────────

    #[test]
    fn render_draws_vertical_scrollbar_when_max_scroll_y_set() {
        let pkg = ScrollViewPackage::new();
        {
            let mut st = pkg.state.write().unwrap();
            st.max_scroll_y = 800.0;
            st.scroll_y = 200.0;
        }

        let mut renderer = MockRenderer::default();
        pkg.on_render(&mut renderer, layout_rect(), None);

        assert!(
            renderer.rects_drawn >= 1,
            "must draw at least one scrollbar rect"
        );
    }

    #[test]
    fn render_draws_page_dots_in_pager_mode() {
        let pkg = ScrollViewPackage::new();
        {
            let mut st = pkg.state.write().unwrap();
            st.snap_x = Some(400.0);
            st.page_count = 4;
        }

        let mut renderer = MockRenderer::default();
        pkg.on_render(&mut renderer, layout_rect(), None);

        assert_eq!(
            renderer.circles_drawn, 4,
            "must draw one dot per page (4 dots)"
        );
    }

    #[test]
    fn render_clip_stack_is_balanced() {
        let pkg = ScrollViewPackage::new();
        let mut renderer = MockRenderer::default();
        pkg.on_render(&mut renderer, layout_rect(), None);

        assert_eq!(
            renderer.clip_depth, 0,
            "push/pop clip rect calls must be balanced, net depth = {}",
            renderer.clip_depth
        );
    }

    #[test]
    fn render_transform_stack_is_balanced() {
        let pkg = ScrollViewPackage::new();
        let mut renderer = MockRenderer::default();
        pkg.on_render(&mut renderer, layout_rect(), None);

        assert_eq!(
            renderer.transform_depth, 0,
            "push/pop transform calls must be balanced"
        );
    }

    // ── apply_scroll_delta ───────────────────────────────────────────────

    #[test]
    fn apply_scroll_delta_updates_position_and_velocity() {
        let pkg = ScrollViewPackage::new();
        pkg.apply_scroll_delta(50.0, 20.0, 300.0, 100.0);

        let st = pkg.state.read().unwrap();
        assert!((st.scroll_x - 50.0).abs() < f32::EPSILON);
        assert!((st.scroll_y - 20.0).abs() < f32::EPSILON);
        assert!((st.velocity_x - 300.0).abs() < f32::EPSILON);
        assert!((st.velocity_y - 100.0).abs() < f32::EPSILON);
    }
}
