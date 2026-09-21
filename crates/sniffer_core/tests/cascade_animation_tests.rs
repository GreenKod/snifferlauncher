use sniffer_core::anim::TransitionManager;
use sniffer_core::style::{Easing, Style, Transition};

#[test]
fn test_staggered_cascade_delay_ordering() {
    let mut manager = TransitionManager::default();

    let style_0 = Style {
        opacity: 0.0,
        transition: Transition {
            duration: 0.20,
            easing: Easing::Linear,
            delay: 0.0,
            stagger_index: 0,
            stagger_interval: 0.05,
        },
        ..Default::default()
    };

    let mut style_1 = style_0.clone();
    style_1.transition.stagger_index = 1;

    let mut style_2 = style_0.clone();
    style_2.transition.stagger_index = 2;

    manager.update_target("item_0", &style_0);
    manager.update_target("item_1", &style_1);
    manager.update_target("item_2", &style_2);

    // Now trigger transition to opacity: 1.0
    let mut target_0 = style_0.clone();
    target_0.opacity = 1.0;
    let mut target_1 = style_1.clone();
    target_1.opacity = 1.0;
    let mut target_2 = style_2.clone();
    target_2.opacity = 1.0;

    manager.update_target("item_0", &target_0);
    manager.update_target("item_1", &target_1);
    manager.update_target("item_2", &target_2);

    assert!(manager.is_animating());

    // Step 0.03s: item_0 is animating, item_1 (delay 0.05s) and item_2 (delay 0.10s) must still be at 0.0
    manager.tick(0.03);
    assert!(manager.get_current_style("item_0").unwrap().opacity > 0.0);
    assert_eq!(manager.get_current_style("item_1").unwrap().opacity, 0.0);
    assert_eq!(manager.get_current_style("item_2").unwrap().opacity, 0.0);

    // Step another 0.04s (total 0.07s): item_1 has now started (> 0.05s), item_2 is still at 0.0 (< 0.10s)
    manager.tick(0.04);
    assert!(manager.get_current_style("item_0").unwrap().opacity > 0.0);
    assert!(manager.get_current_style("item_1").unwrap().opacity > 0.0);
    assert_eq!(manager.get_current_style("item_2").unwrap().opacity, 0.0);

    // Run until all settled
    let mut steps = 0;
    while manager.is_animating() && steps < 600 {
        manager.tick(1.0 / 60.0);
        steps += 1;
    }

    assert!(!manager.is_animating());
    assert_eq!(manager.get_current_style("item_0").unwrap().opacity, 1.0);
    assert_eq!(manager.get_current_style("item_1").unwrap().opacity, 1.0);
    assert_eq!(manager.get_current_style("item_2").unwrap().opacity, 1.0);
}

#[test]
fn test_spring_easing_cascade_convergence() {
    let mut manager = TransitionManager::default();

    let start_style = Style {
        opacity: 0.0,
        transform: sniffer_core::style::Transform {
            translate_y: 100.0,
            ..Default::default()
        },
        transition: Transition {
            duration: 0.35,
            easing: Easing::Spring {
                stiffness: 220.0,
                damping: 20.0,
            },
            delay: 0.0,
            stagger_index: 0,
            stagger_interval: 0.02,
        },
        ..Default::default()
    };

    manager.update_target("card_0", &start_style);

    let mut target_style = start_style.clone();
    target_style.transform.translate_y = 0.0;
    target_style.opacity = 1.0;

    manager.update_target("card_0", &target_style);
    assert!(manager.is_animating());

    let dt = 1.0 / 120.0;
    let mut steps = 0;
    while manager.is_animating() && steps < 1200 {
        manager.tick(dt);
        steps += 1;
    }

    assert!(!manager.is_animating());
    let current = manager.get_current_style("card_0").unwrap();
    assert!((current.transform.translate_y - 0.0).abs() < 1e-2);
    assert!((current.opacity - 1.0).abs() < 1e-2);
}
