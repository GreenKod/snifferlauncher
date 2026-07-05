// =============================================================================
// Sniffer UI Plugin: Default Interface (Redesigned)
// =============================================================================

let state = {
    greetingText: "Sniffer Dashboard",
    bgColor: "#121212", // Dark theme background
    clickCount: 0,
    searchQuery: "",
    isSearchFocused: false,
    fabOpacity: 0.9,
    scrollY: 0.0,
    items: [1, 2, 3, 4, 5, 6, 7, 8],
    boxToggled: false,
};

function App() {
    return ScrollView("root_scroll", {
        scroll_y: state.scrollY,
        style: {
            display: "Flex",
            flex_direction: "Column",
            width: pct(100),
            height: pct(100),
            background_color: hex(state.bgColor),
            align_items: "Center",
            padding: padXY(vh(8.0), vw(5.0)), // Padding top/bottom and left/right
            gap: vh(4.0),
        }
    }, [
        // --- Hero Header ---
        Container("header_container", {
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
        ]),

        // --- Search Bar ---
        TextInput("search_input", state.searchQuery || "Search plugins...", {
            focused: state.isSearchFocused,
            style: {
                text_color: hex("#FFFFFF"),
                text_size: vmin(4.5), // Yazı boyutunu vmin ile sabitliyoruz
                width: pct(90), // Çubuk genişliği ekrana yayılacak
                height: px(vmin(12.0)), // Yüksekliği sabit veriyoruz çünkü Auto çalışmıyor
                background_color: hex("#1E1E1E"),
                border_radius: vmin(3.0),
                border_width: vmin(0.3),
                border_color: state.isSearchFocused ? hex("#FF0055") : hex("#333333"),
                padding: pad(vmin(3.0)), // Her yönden eşit boşluk
                shadow_color: hex("#88000000"),
                shadow_offset_y: vmin(1.0),
                shadow_spread: vmin(1.0),
                overflow_hidden: true,
                flex_shrink: 0.0, // Taffy'nin dikeyde sıkıştırmasını engelle
                transition: { duration: 0.3, easing: "ease_out" },
                transform: {
                    scale: state.isSearchFocused ? 1.02 : 1.0,
                    rotate: 0.0,
                    translate_x: 0.0,
                    translate_y: 0.0,
                }
            },
        }),

        // --- Quick Actions Grid (Horizontal layout approximation) ---
        Container("actions_grid", {
            width: pct(90),
            height: "Auto",
            flex_direction: "Row",
            flex_wrap: "Wrap",
            justify_content: "SpaceBetween",
            gap: vmin(3.0),
        }, [
            // Counter Box
            Container("counter_box", {
                width: px(vw(42.0)), // Roughly half width minus gap
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
        ]),

        // --- List View Header ---
        Container("list_header", {
            width: pct(90),
            height: "Auto",
            justify_content: "Start",
            align_items: "Start",
            padding: padXY(vmin(2.0), vmin(0.0)),
        }, [
            Label("list_title", "Recent Modules", {
                text_color: hex("#FFFFFF"),
                text_size: vmin(5.0),
                width: "Auto",
            }),
        ]),

        // --- List Items ---
        ...state.items.map((n, i) =>
            Container("list_item_" + i, {
                width: pct(90),
                height: px(vmin(15.0)),
                background_color: hex("#1E1E1E"),
                border_radius: vmin(3.0),
                flex_direction: "Row",
                align_items: "Center",
                justify_content: "Start",
                padding: padXY(vmin(0.0), vmin(4.0)),
                gap: vmin(4.0),
            }, [
                // Icon placeholder
                Container("item_icon_" + i, {
                    width: px(vmin(8.0)),
                    height: px(vmin(8.0)),
                    border_radius: vmin(4.0),
                    background_color: hex("#FF0055"),
                }, []),
                // Text
                Label("item_text_" + i, "Module " + n, {
                    text_size: vmin(4.5),
                    text_color: hex("#DDDDDD"),
                    width: "Auto",
                })
            ])
        ),

        // --- Bottom Padding for FAB ---
        Container("bottom_pad", {
            width: pct(100),
            height: px(vmin(20.0)),
        }, []),

        // --- FAB (+) Button (Positioned Absolute inside ScrollView) ---
        // Note: Absolute positioned elements inside ScrollView scroll with content.
        // If we want it sticky, we'd need a Container wrapping ScrollView.
        // For now, let's keep it in the ScrollView flow or wrap the App in a Container.
    ]);
}

// Wrap the App to provide a sticky FAB
function Root() {
    return Container("root", {
        width: pct(100),
        height: pct(100),
        position: "Relative",
        background_color: hex(state.bgColor),
    }, [
        App(),
        // Sticky FAB
        Container("fab_button", {
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
        ]),
    ]);
}


// Event Listener
globalThis.onEvent = function (eventJsonString) {
    const e = JSON.parse(eventJsonString);

    // Greeting label clicked
    if (e.type === "Click" && e.id === host_hash("btn-greeting")) {
        SnifferUI.setState({
            greetingText: "Ready for Action! 🚀",
            bgColor: "#111111",
        });
        return "[]";
    }

    // Counter clicked
    if (e.type === "Click" && (e.id === host_hash("counter_box") || e.id === host_hash("btn-counter") || e.id === host_hash("counter_val"))) {
        SnifferUI.setState({ clickCount: state.clickCount + 1 });
        return "[]";
    }

    // Animated box clicked
    if (e.type === "Click" && (e.id === host_hash("anim_box") || e.id === host_hash("anim_box_text"))) {
        SnifferUI.setState({ boxToggled: !state.boxToggled });
        return "[]";
    }

    // Search input clicked → focus
    if (e.type === "Click" && e.id === host_hash("search_input")) {
        host_focus_input("search_input");
        const query = state.searchQuery === "" ? "" : state.searchQuery;
        SnifferUI.setState({ isSearchFocused: true, searchQuery: query });
        return "[]";
    }

    // FAB pressed → dim effect
    if (e.type === "PointerDown" &&
        (String(e.id) === String(host_hash("fab_button")) ||
            String(e.id) === String(host_hash("fab_text")))) {
        SnifferUI.setState({ fabOpacity: 0.5 });
        return "[]";
    }

    // Pointer released
    if (e.type === "PointerUp") {
        const wasFabPressed = state.fabOpacity === 0.5;
        if (wasFabPressed) {
            // FAB released → add new item
            const newItems = [...state.items, state.items.length + 1];
            SnifferUI.setState({ fabOpacity: 0.9, items: newItems });
        } else {
            SnifferUI.setState({ fabOpacity: 0.9 });
        }
        return "[]";
    }

    // Clicked outside → blur
    if (e.type === "ClickOutside") {
        host_blur_input();
        SnifferUI.setState({ isSearchFocused: false });
        return "[]";
    }

    // Text input
    if (e.type === "TextInput") {
        SnifferUI.setState({ searchQuery: state.searchQuery + e.text });
        return "[]";
    }

    // Backspace
    if (e.type === "Backspace") {
        const val = state.searchQuery;
        if (val.length > 0) {
            SnifferUI.setState({ searchQuery: val.slice(0, -1) });
        }
        return "[]";
    }

    // Scroll
    if (e.type === "Scroll" && e.id === host_hash("root_scroll")) {
        const s_y = state.scrollY + e.dy;
        const max_scroll = e.max_y ?? 999999;
        SnifferUI.setState({ scrollY: Math.max(0, Math.min(s_y, max_scroll)) });
        return "[]";
    }

    // Window Resize
    if (e.type === "WindowResized") {
        SnifferUI.forceUpdate();
        return "[]";
    }

    return "[]";
};

// Start the SnifferUI framework with our root component and initial state
SnifferUI.start(Root, state);
