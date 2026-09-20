use sniffer_core::anim::{AnimState, TransitionManager};
use sniffer_core::style::{Easing, StyleBuilder, Transition};

#[test]
fn test_elevation_pressed_state_transition() {
    // Resting state: elevated card with elevation 8.0, scale 1.0
    let resting_style = StyleBuilder::new()
        .elevation(8.0)
        .shadow_blur(16.0)
        .scale(1.0)
        .transition(Transition {
            duration: 0.15, // 150ms press animation
            easing: Easing::EaseOut,
        })
        .build();

    // Pressed state: card depresses to elevation 2.0, scale 0.96
    let pressed_style = StyleBuilder::new()
        .elevation(2.0)
        .shadow_blur(4.0)
        .scale(0.96)
        .transition(Transition {
            duration: 0.15,
            easing: Easing::EaseOut,
        })
        .build();

    let mut state = AnimState {
        start_style: resting_style.clone(),
        target_style: pressed_style,
        current_style: resting_style,
        time_elapsed: 0.0,
        is_active: true,
    };

    // Before update
    assert_eq!(state.current_style.elevation, 8.0);
    assert_eq!(state.current_style.shadow_blur, 16.0);
    assert_eq!(state.current_style.transform.scale, 1.0);

    // Step halfway (dt = 0.024 * 3 approx or simulate 75ms)
    // Update multiple small frames
    state.update(0.02);
    state.update(0.02);
    state.update(0.02);

    // At 60ms / 150ms = 40% progress, elevation should be between 2.0 and 8.0
    assert!(
        state.current_style.elevation < 8.0,
        "elevation should decrease towards pressed state"
    );
    assert!(
        state.current_style.elevation > 2.0,
        "elevation should not yet reach final pressed state"
    );
    assert!(
        state.current_style.shadow_blur < 16.0,
        "shadow blur should shrink during press"
    );
    assert!(
        state.current_style.transform.scale < 1.0,
        "scale should shrink during press"
    );

    // Advance to completion
    for _ in 0..10 {
        state.update(0.02);
    }

    assert!(
        !state.is_active,
        "animation state should deactivate upon reaching target duration"
    );
    assert!(
        (state.current_style.elevation - 2.0).abs() < 1e-4,
        "elevation reaches target pressed level"
    );
    assert!(
        (state.current_style.shadow_blur - 4.0).abs() < 1e-4,
        "shadow blur reaches target pressed level"
    );
    assert!(
        (state.current_style.transform.scale - 0.96).abs() < 1e-4,
        "scale reaches target pressed scale"
    );
}

#[test]
fn test_transition_manager_elevation_lifecycle() {
    let mut manager = TransitionManager::default();

    let initial = StyleBuilder::new()
        .elevation(6.0)
        .shadow_blur(12.0)
        .shadow_spread(2.0)
        .shadow_offset_y(4.0)
        .shadow_color(0x6000_0000)
        .scale(1.0)
        .build();

    let card_id = "app_card_42";
    manager.update_target(card_id, &initial);

    // Initial state is inactive with instant apply
    assert!(!manager.is_animating());
    let current = manager.get_current_style(card_id).unwrap();
    assert_eq!(current.elevation, 6.0);
    assert_eq!(current.shadow_blur, 12.0);

    // Trigger pressed state with 100ms transition
    let pressed = StyleBuilder::new()
        .elevation(1.5)
        .shadow_blur(3.0)
        .shadow_spread(0.5)
        .shadow_offset_y(1.0)
        .shadow_color(0x3000_0000)
        .scale(0.97)
        .transition(Transition {
            duration: 0.1,
            easing: Easing::Linear,
        })
        .build();

    manager.update_target(card_id, &pressed);
    assert!(manager.is_animating());

    // Tick by 50ms (dt = 0.024 twice + 0.002)
    let needs_redraw = manager.tick(0.024);
    assert!(needs_redraw);
    let _ = manager.tick(0.024);
    let _ = manager.tick(0.002);

    let mid = manager.get_current_style(card_id).unwrap();
    // At t=0.5 linear:
    // elevation = 6.0 + 0.5 * (1.5 - 6.0) = 3.75
    assert!((mid.elevation - 3.75).abs() < 0.1);
    // shadow_blur = 12.0 + 0.5 * (3.0 - 12.0) = 7.5
    assert!((mid.shadow_blur - 7.5).abs() < 0.1);
    // scale = 1.0 + 0.5 * (0.97 - 1.0) = 0.985
    assert!((mid.transform.scale - 0.985).abs() < 0.005);

    // Complete transition
    while manager.is_animating() {
        manager.tick(0.024);
    }

    let end = manager.get_current_style(card_id).unwrap();
    assert!((end.elevation - 1.5).abs() < 1e-4);
    assert!((end.shadow_blur - 3.0).abs() < 1e-4);
    assert!((end.shadow_spread - 0.5).abs() < 1e-4);
    assert!((end.shadow_offset_y - 1.0).abs() < 1e-4);
    assert!((end.transform.scale - 0.97).abs() < 1e-4);
    assert!(!manager.tick(0.016), "no redraw when animations are idle");
}

#[test]
fn test_border_gradient_interpolation() {
    let top_start = 0xFFFF_0000; // pure red
    let bot_start = 0xFF00_FF00; // pure green
    let top_end = 0xFF00_00FF; // pure blue
    let bot_end = 0xFF00_0000; // pure black

    let s1 = StyleBuilder::new()
        .border_gradient(top_start, bot_start)
        .transition(Transition {
            duration: 0.1,
            easing: Easing::Linear,
        })
        .build();

    let s2 = StyleBuilder::new()
        .border_gradient(top_end, bot_end)
        .transition(Transition {
            duration: 0.1,
            easing: Easing::Linear,
        })
        .build();

    let mut state = AnimState {
        start_style: s1.clone(),
        target_style: s2,
        current_style: s1,
        time_elapsed: 0.0,
        is_active: true,
    };

    // Update to 50ms / 100ms (t=0.5)
    state.update(0.024);
    state.update(0.024);
    state.update(0.002);

    let (cur_top, cur_bot) = state.current_style.border_gradient.unwrap();
    // Top interpolated red -> blue (halfway red and halfway blue)
    let red_top = (cur_top >> 16) & 0xff;
    let blue_top = cur_top & 0xff;
    assert!((i64::from(red_top) - 127).abs() <= 2);
    assert!((i64::from(blue_top) - 127).abs() <= 2);

    // Bottom interpolated green -> black (halfway green)
    let green_bot = (cur_bot >> 8) & 0xff;
    assert!((i64::from(green_bot) - 127).abs() <= 2);

    // Advance to end
    while state.is_active {
        state.update(0.024);
    }
    let (final_top, final_bot) = state.current_style.border_gradient.unwrap();
    assert_eq!(final_top, top_end);
    assert_eq!(final_bot, bot_end);
}
