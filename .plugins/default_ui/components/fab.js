// FAB Button Component

function FabButtonComponent() {
    return Container("fab_button", {
        position: "Absolute",
        bottom: px(vmin(8.0)),
        right: px(vmin(8.0)),
        width: px(vmin(14.0)),
        height: px(vmin(14.0)),
        border_radius: vmin(7.0),
        background_gradient: [hex("#FF0055"), hex("#FF00AA")],
        shadow_color: hex("#AA000000"),
        shadow_spread: vmin(0.5),
        shadow_offset_y: vmin(1.0),
        opacity: state.fabOpacity,
        justify_content: "Center",
        align_items: "Center",
        transition: { duration: 0.2, easing: "ease_out" },
        transform: {
            scale: state.fabOpacity === 0.5 ? 0.8 : 1.0,
            rotate: state.fabOpacity === 0.5 ? 45.0 : 0.0,
            translate_x: 0.0,
            translate_y: 0.0,
        }
    }, [
        Label("fab_text", "+", {
            text_color: hex("#FFFFFF"),
            text_size: vmin(8.0),
        }),
    ]);
}
