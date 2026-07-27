// Actions Grid Component

function ActionsGridComponent() {
    return Container("actions_grid", {
        width: pct(90),
        height: "Auto",
        flex_direction: "Row",
        flex_wrap: "Wrap",
        justify_content: "SpaceBetween",
        gap: vmin(3.0),
    }, [
        // Counter Box
        Container("counter_box", {
            width: px(vw(42.0)),
            height: px(vw(42.0)),
            background_gradient: [hex("#2a0845"), hex("#6441A5")],
            border_radius: vmin(4.0),
            justify_content: "Center",
            align_items: "Center",
            shadow_color: hex("#55000000"),
            shadow_offset_y: vmin(1.0),
            shadow_spread: vmin(1.0),
        }, [
            Label("btn-counter", "Clicks", {
                text_color: hex("#DDDDDD"),
                text_size: vmin(4.0),
                width: "Auto",
            }),
            Label("counter_val", String(state.clickCount), {
                text_color: hex("#FFFFFF"),
                text_size: vmin(12.0),
                width: "Auto",
            }),
        ]),

        // Animated Spring Box
        Container("anim_box", {
            width: px(vw(42.0)),
            height: px(vw(42.0)),
            background_gradient: state.boxToggled ? [hex("#FF416C"), hex("#FF4B2B")] : [hex("#1D2B64"), hex("#F8CDDA")],
            border_radius: state.boxToggled ? vmin(20.0) : vmin(4.0),
            justify_content: "Center",
            align_items: "Center",
            shadow_color: hex("#55000000"),
            shadow_offset_y: vmin(1.0),
            shadow_spread: vmin(1.0),
            transition: { duration: 0.8, easing: { spring: { stiffness: 120, damping: 12 } } },
            transform: {
                scale: state.boxToggled ? 1.05 : 1.0,
                rotate: state.boxToggled ? 180.0 : 0.0,
                translate_x: 0.0,
                translate_y: 0.0,
            },
        }, [
            Label("anim_box_text", "Spring!", {
                text_color: hex("#FFFFFF"),
                text_size: vmin(5.0),
            })
        ]),
    ]);
}
