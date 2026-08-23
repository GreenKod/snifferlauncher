use super::constants::{RUBBER_BAND_RESTORING, SNAP_VELOCITY_THRESHOLD, SPRING_STIFFNESS};
use super::state::ScrollState;
use sniffer_core::vault::DataVault;

pub fn advance_simulation(st: &mut ScrollState, vault: &DataVault, dt_secs: f32) {
    if dt_secs <= f32::EPSILON {
        return;
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
                if let Some(ref cb) = st.on_snap_callback {
                    let _ = vault.set(
                        "pkg.scroll_view.on_snap",
                        &format!(r#"{{"callback":"{cb}","page":{new_page}}}"#),
                        "com.sniffer.scroll_view",
                    );
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
}
