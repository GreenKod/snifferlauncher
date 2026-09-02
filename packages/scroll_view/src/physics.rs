use super::constants::{RUBBER_BAND_RESTORING, SNAP_VELOCITY_THRESHOLD, SPRING_STIFFNESS};
use super::state::ScrollState;

/// Result of a single physics tick.
///
/// When a snap-to-page transition completes the caller receives the vault key,
/// value and authority that should be written — keeping this module free of any
/// `DataVault` dependency.
pub struct PhysicsResult {
    /// `Some((key, value, authority))` when a new page was snapped to.
    pub snap_event: Option<(String, String, &'static str)>,
}

/// Advance the scroll physics simulation by `dt_secs` seconds.
///
/// Returns a [`PhysicsResult`] that the caller should act on (e.g. write the
/// snap event into the vault).  The function itself is pure — it only reads and
/// mutates `ScrollState` without touching any external subsystem.
pub fn advance_simulation(st: &mut ScrollState, dt_secs: f32) -> PhysicsResult {
    let mut result = PhysicsResult { snap_event: None };

    if dt_secs <= f32::EPSILON {
        return result;
    }

    // 1. Momentum damping
    let decay = (1.0 - st.momentum_damping).powf(dt_secs * 60.0);
    st.velocity_x *= decay;
    st.velocity_y *= decay;

    // 2. Integrate position
    st.scroll_x += st.velocity_x * dt_secs;
    st.scroll_y += st.velocity_y * dt_secs;

    // 3. Boundary handling
    let rb = st.rubber_band;

    if rb > f32::EPSILON {
        if st.scroll_x < 0.0 {
            st.velocity_x += (-st.scroll_x) * rb * RUBBER_BAND_RESTORING * dt_secs;
        } else if st.max_scroll_x > 0.0 && st.scroll_x > st.max_scroll_x {
            st.velocity_x -= (st.scroll_x - st.max_scroll_x) * rb * RUBBER_BAND_RESTORING * dt_secs;
        }

        if st.scroll_y < 0.0 {
            st.velocity_y += (-st.scroll_y) * rb * RUBBER_BAND_RESTORING * dt_secs;
        } else if st.max_scroll_y > 0.0 && st.scroll_y > st.max_scroll_y {
            st.velocity_y -= (st.scroll_y - st.max_scroll_y) * rb * RUBBER_BAND_RESTORING * dt_secs;
        }
    } else {
        st.scroll_x = st.scroll_x.clamp(0.0, st.max_scroll_x.max(0.0));
        st.scroll_y = st.scroll_y.clamp(0.0, st.max_scroll_y.max(0.0));
        if st.scroll_x == 0.0 || st.scroll_x == st.max_scroll_x {
            st.velocity_x = 0.0;
        }
        if st.scroll_y == 0.0 || st.scroll_y == st.max_scroll_y {
            st.velocity_y = 0.0;
        }
    }

    // 4. Snap-to-page (X axis)
    if let Some(snap_x) = st.snap_x {
        if snap_x > f32::EPSILON && st.velocity_x.abs() < SNAP_VELOCITY_THRESHOLD {
            let max_page = st.page_count.saturating_sub(1) as f32;
            let target_page = (st.scroll_x / snap_x).round().clamp(0.0, max_page);
            let target_x = target_page * snap_x;

            let delta = (target_x - st.scroll_x) * SPRING_STIFFNESS * dt_secs;
            st.scroll_x += delta;

            let new_page = target_page as i32;
            if new_page != st.current_page && (st.scroll_x - target_x).abs() < 1.0 {
                st.current_page = new_page;
                // Notify caller so it can write the snap event to the vault
                // without this module depending on DataVault.
                if let Some(ref cb) = st.on_snap_callback {
                    result.snap_event = Some((
                        "pkg.scroll_view.on_snap".to_string(),
                        format!(r#"{{"callback":"{cb}","page":{new_page}}}"#),
                        "com.sniffer.scroll_view",
                    ));
                }
            }
        }
    }

    // 4b. Snap-to-page (Y axis)
    if let Some(snap_y) = st.snap_y {
        if snap_y > f32::EPSILON && st.velocity_y.abs() < SNAP_VELOCITY_THRESHOLD {
            let max_page = st.page_count.saturating_sub(1) as f32;
            let target_page = (st.scroll_y / snap_y).round().clamp(0.0, max_page);
            let target_y = target_page * snap_y;
            st.scroll_y += (target_y - st.scroll_y) * SPRING_STIFFNESS * dt_secs;
        }
    }

    result
}
