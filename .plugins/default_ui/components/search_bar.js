// Search Bar Component

function SearchBarComponent() {
    return TextInput("search_input", state.searchQuery || "Search plugins...", {
        focused: state.isSearchFocused,
        style: {
            text_color: hex("#FFFFFF"),
            text_size: vmin(4.5),
            width: pct(90),
            height: px(vmin(12.0)),
            background_color: hex("#1E1E1E"),
            border_radius: vmin(3.0),
            border_width: vmin(0.3),
            border_color: state.isSearchFocused ? hex("#FF0055") : hex("#333333"),
            padding: pad(vmin(3.0)),
            shadow_color: hex("#88000000"),
            shadow_offset_y: vmin(1.0),
            shadow_spread: vmin(1.0),
            overflow_hidden: true,
            flex_shrink: 0.0,
            transition: { duration: 0.3, easing: "ease_out" },
            transform: {
                scale: state.isSearchFocused ? 1.02 : 1.0,
                rotate: 0.0,
                translate_x: 0.0,
                translate_y: 0.0,
            }
        },
    });
}
