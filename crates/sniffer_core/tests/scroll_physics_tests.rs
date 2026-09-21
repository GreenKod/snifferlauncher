use sniffer_core::physics::ScrollPhysics;

#[test]
fn test_scroll_physics_snap_spring_integration() {
    let mut phys = ScrollPhysics {
        snap_x: Some(360.0),
        page_count: Some(3),
        ..Default::default()
    };

    // Position at page 0 with forward velocity towards page 1
    phys.pos_x = 100.0;
    phys.release_drag(400.0);

    assert!(phys.snap_target_x.is_some());
    assert_eq!(phys.snap_target_x, Some(360.0));
    assert!(phys.spring_sim_x.is_some());
    assert!(phys.is_animating());

    // Step physics at 120 FPS
    let dt = 1.0 / 120.0;
    let mut steps = 0;
    while phys.tick(dt) && steps < 600 {
        steps += 1;
    }

    assert_eq!(phys.pos_x, 360.0);
    assert_eq!(phys.vel_x, 0.0);
    assert!(phys.snap_target_x.is_none());
    assert!(phys.spring_sim_x.is_none());
    assert!(!phys.is_animating());
}

#[test]
fn test_scroll_physics_rubber_band_y_spring_recovery() {
    let mut phys = ScrollPhysics {
        max_y: Some(1000.0),
        ..Default::default()
    };

    // Overscroll past top boundary
    phys.pos_y = -120.0;
    phys.release_drag_y(0.0);

    assert!(phys.spring_sim_y.is_some());
    assert!(phys.is_animating());

    let dt = 1.0 / 120.0;
    let mut steps = 0;
    while phys.tick(dt) && steps < 600 {
        steps += 1;
    }

    assert_eq!(phys.pos_y, 0.0);
    assert_eq!(phys.vel_y, 0.0);
    assert!(phys.spring_sim_y.is_none());
    assert!(!phys.is_animating());

    // Overscroll past bottom boundary
    phys.pos_y = 1150.0;
    phys.release_drag_y(0.0);

    assert!(phys.spring_sim_y.is_some());
    assert!(phys.is_animating());

    steps = 0;
    while phys.tick(dt) && steps < 600 {
        steps += 1;
    }

    assert_eq!(phys.pos_y, 1000.0);
    assert_eq!(phys.vel_y, 0.0);
    assert!(phys.spring_sim_y.is_none());
}

#[test]
fn test_scroll_physics_drag_cancels_active_spring() {
    let mut phys = ScrollPhysics {
        max_y: Some(1000.0),
        ..Default::default()
    };

    phys.pos_y = -80.0;
    phys.release_drag_y(0.0);
    assert!(phys.spring_sim_y.is_some());

    // User touches screen and drags again
    phys.apply_drag_y(10.0);
    assert!(phys.spring_sim_y.is_none());
}
