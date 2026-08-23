use super::*;
use sniffer_core::vault::DataVault;
use sniffer_pkg::error::PackageError;
use std::sync::Arc;

mod render_tests;

fn make_vault() -> Arc<DataVault> {
    Arc::new(DataVault::default())
}

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
    pkg.apply_descriptor(r#"{"snap_x":400.0}"#).unwrap();
    assert!(pkg.state.read().unwrap().snap_x.is_some());

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

#[test]
fn momentum_decays_to_near_zero_after_many_frames() {
    let pkg = ScrollViewPackage::new();
    let vault = make_vault();

    {
        let mut st = pkg.state.write().unwrap();
        st.velocity_x = 1000.0;
        st.max_scroll_x = 100_000.0;
    }

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
        st.velocity_x = 100.0;
        st.max_scroll_x = 100_000.0;
        st.momentum_damping = 0.0;
    }

    pkg.on_update(&vault, 0.1);
    let (x, _) = pkg.scroll_position();
    assert!((x - 10.0).abs() < 0.5, "expected scroll_x ≈ 10.0, got {x}");
}

#[test]
fn rubber_band_returns_overshoot_toward_zero() {
    let pkg = ScrollViewPackage::new();
    let vault = make_vault();

    {
        let mut st = pkg.state.write().unwrap();
        st.scroll_x = -50.0;
        st.velocity_x = 0.0;
        st.rubber_band = 0.5;
        st.momentum_damping = 0.0;
    }

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
        st.scroll_x = -200.0;
        st.velocity_x = -500.0;
        st.rubber_band = 0.0;
        st.max_scroll_x = 1000.0;
    }

    pkg.on_update(&vault, 1.0 / 60.0);

    let (x, _) = pkg.scroll_position();
    assert!(
        x >= 0.0,
        "rigid boundary must clamp scroll_x to >= 0, got {x}"
    );
}

#[test]
fn snap_springs_toward_nearest_page() {
    let pkg = ScrollViewPackage::new();
    let vault = make_vault();

    {
        let mut st = pkg.state.write().unwrap();
        st.snap_x = Some(400.0);
        st.page_count = 3;
        st.scroll_x = 210.0;
        st.velocity_x = 0.0;
    }

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
        st.velocity_x = 500.0;
        st.max_scroll_x = 800.0;
        st.momentum_damping = 0.0;
    }

    pkg.on_update(&vault, 1.0 / 60.0);

    let (x, _) = pkg.scroll_position();
    assert!(
        x > 210.0,
        "position must advance with velocity when above snap threshold, got {x}"
    );
}

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
