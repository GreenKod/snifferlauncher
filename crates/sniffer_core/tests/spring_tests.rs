use sniffer_core::anim::{SpringConfig, SpringSimulation};

#[test]
fn test_spring_presets_validity() {
    let presets = [
        SpringConfig::default(),
        SpringConfig::snappy(),
        SpringConfig::bouncy(),
        SpringConfig::gentle(),
        SpringConfig::rubber_band(),
    ];

    for config in presets {
        assert!(config.mass > 0.0);
        assert!(config.stiffness > 0.0);
        assert!(config.damping >= 0.0);
        assert!(config.position_tolerance > 0.0);
        assert!(config.velocity_tolerance > 0.0);

        let mut sim = SpringSimulation::new(config, 0.0, 100.0);
        assert!(!sim.is_at_rest());
        assert_eq!(sim.position(), 0.0);
        assert_eq!(sim.target(), 100.0);

        // Step through simulation at 120 FPS
        let dt = 1.0 / 120.0;
        let mut frames = 0;
        while !sim.is_at_rest() && frames < 1200 {
            let pos = sim.step(dt);
            assert!(!pos.is_nan());
            assert!(!pos.is_infinite());
            frames += 1;
        }

        assert!(
            sim.is_at_rest(),
            "Preset failed to converge within 10s: {config:?}"
        );
        assert_eq!(sim.position(), 100.0);
        assert_eq!(sim.velocity(), 0.0);
    }
}

#[test]
fn test_spring_zero_delta_time_does_not_mutate() {
    let config = SpringConfig::snappy();
    let mut sim = SpringSimulation::new(config, 10.0, 50.0).with_initial_velocity(100.0);
    let pos_before = sim.position();
    let vel_before = sim.velocity();

    sim.step(0.0);
    assert_eq!(sim.position(), pos_before);
    assert_eq!(sim.velocity(), vel_before);

    sim.step(-0.016);
    assert_eq!(sim.position(), pos_before);
    assert_eq!(sim.velocity(), vel_before);
}

#[test]
fn test_spring_negative_displacement_and_reverse_motion() {
    let config = SpringConfig::snappy();
    let mut sim = SpringSimulation::new(config, 100.0, -50.0).with_initial_velocity(-200.0);

    let dt = 1.0 / 60.0;
    let mut steps = 0;
    while !sim.is_at_rest() && steps < 600 {
        sim.step(dt);
        steps += 1;
    }

    assert!(sim.is_at_rest());
    assert_eq!(sim.position(), -50.0);
    assert_eq!(sim.velocity(), 0.0);
}

#[test]
fn test_spring_dynamic_retarget_smooth_continuation() {
    let config = SpringConfig::snappy();
    let mut sim = SpringSimulation::new(config, 0.0, 100.0);

    // Run for 5 frames
    for _ in 0..5 {
        sim.step(1.0 / 60.0);
    }

    let pos_interim = sim.position();
    let vel_interim = sim.velocity();
    assert!(pos_interim > 0.0);
    assert!(vel_interim > 0.0);

    // Retarget to 200.0 mid-motion
    sim.set_target(200.0);
    assert_eq!(sim.target(), 200.0);
    assert!(!sim.is_at_rest());

    // Next step must be continuous from pos_interim and vel_interim
    let next_pos = sim.step(1.0 / 60.0);
    assert!(next_pos > pos_interim);

    // Finish simulation
    while !sim.is_at_rest() {
        sim.step(1.0 / 60.0);
    }
    assert_eq!(sim.position(), 200.0);
}
