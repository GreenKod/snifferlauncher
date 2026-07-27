// Header Component

function HeaderComponent() {
    return Container("header_container", {
        width: pct(90),
        height: "Auto",
        flex_direction: "Column",
        align_items: "Center",
        justify_content: "Center",
        gap: vh(1.0),
    }, [
        Label("btn-greeting", state.greetingText, {
            text_color: hex("#FFFFFF"),
            text_size: vmin(8.0),
            width: "Auto",
        }),
        Label("subtitle", "System is running smoothly.", {
            text_color: hex("#A0A0A0"),
            text_size: vmin(4.0),
            width: "Auto",
        }),
    ]);
}
