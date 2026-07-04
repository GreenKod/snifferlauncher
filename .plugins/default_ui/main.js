// =============================================================================
// default_ui — main.js  (Written using the SnifferUI framework)
// =============================================================================
// The framework (sniffer_ui.js) is automatically loaded before this file
// via the `preload` field in manifest.json. Helpers like Container(), Label(),
// ScrollView(), hex(), px(), pct(), and SnifferUI are directly available.
// =============================================================================

// ---------------------------------------------------------------------------
// 1. STATE
// ---------------------------------------------------------------------------
let state = {
    greetingText:    "Sniffer Dashboard",
    bgColor:         "#FFD700",
    clickCount:      0,
    searchQuery:     "",
    isSearchFocused: false,
    fabOpacity:      0.9,
    scrollY:         0.0,
    items:           [1, 2, 3, 4, 5, 6],
};

// ---------------------------------------------------------------------------
// 2. RENDER FUNCTION — Always renders the UI based on the current state
// ---------------------------------------------------------------------------
function App() {
    return Container("root", {
        display:          "Flex",
        flex_direction:   "Column",
        justify_content:  "Center",
        align_items:      "Center",
        width:            pct(100),
        height:           pct(100),
        background_color: hex(state.bgColor),
        gap:              vh(2.0),
    }, [
        // --- Greeting Label ---
        Label("btn-greeting", state.greetingText, {
            text_color: hex("#000000"),
            text_size:  vmin(8.0),
            width:      "Auto",
        }),

        // --- Click Counter ---
        Label("btn-counter", "Clicks: " + state.clickCount, {
            text_color: hex("#000000"),
            text_size:  vmin(5.0),
            width:      "Auto",
        }),

        // --- ScrollView ---
        ScrollView("scroll-container", {
            scroll_y: state.scrollY,
            style: {
                width:            pct(90),
                height:           pct(50),
                background_color: hex("#FFFFFF"),
                overflow_hidden:  true,
                border_radius:    vmin(2.0),
                gap:              vh(1.0),
            },
        }, [
            // Header Image
            Image("test_img", ".plugins/default_ui/test.jpg", {
                width:        pct(100),
                height:       pct(40),
                border_radius: vmin(2.0),
                object_fit:   "Cover",
            }),

            // Dynamic List
            ...state.items.map((n, i) =>
                Label("scroll-item-" + i, "List Item " + n, {
                    text_size:  vmin(5.0),
                    text_color: hex("#000000"),
                    padding:    pad(vmin(2.0)),
                    height:     "Auto",
                })
            ),
        ]),

        // --- Search Input ---
        TextInput("search_input", state.searchQuery || "Search plugins...", {
            focused: state.isSearchFocused,
            style: {
                text_color:       hex("#0000FF"),
                text_size:        vmin(4.0),
                width:            pct(80),
                height:           "Auto",
                background_color: hex("#E0E0E0"),
                border_radius:    vmin(1.5),
                padding:          pad(vmin(2.0)),
            },
        }),

        // --- FAB (+) Button ---
        Container("fab_button", {
            position:             "Absolute",
            bottom:               px(vmin(5.0)),
            right:                px(vmin(5.0)),
            width:                px(vmin(12.0)),
            height:               px(vmin(12.0)),
            border_radius:        vmin(6.0),
            background_gradient:  [hex("#FF0055"), hex("#FF00AA")],
            shadow_color:         hex("#88000000"),
            shadow_spread:        vmin(0.5),
            shadow_offset_y:      vmin(0.5),
            opacity:              state.fabOpacity,
            justify_content:      "Center",
            align_items:          "Center",
        }, [
            Label("fab_text", "+", {
                text_color: hex("#FFFFFF"),
                text_size:  vmin(7.0),
            }),
        ]),
    ]);
}

// ---------------------------------------------------------------------------
// 3. EVENTS
// ---------------------------------------------------------------------------
globalThis.onEvent = function (eventJsonString) {
    const e = JSON.parse(eventJsonString);

    // Greeting label clicked
    if (e.type === "Click" && e.id === host_hash("btn-greeting")) {
        SnifferUI.setState({
            greetingText: "Active! 🎉",
            bgColor:      "#00CC66",
        });
        return "[]";
    }

    // Counter clicked
    if (e.type === "Click" && e.id === host_hash("btn-counter")) {
        SnifferUI.setState({ clickCount: state.clickCount + 1 });
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
    if (e.type === "Scroll" && e.id === host_hash("scroll-container")) {
        // Calculate dynamic content height (Approximation)
        // Image = 40vh, Item = 5vmin text + 4vmin padding + 1vh gap
        const imgH = vh(40);
        const itemH = vmin(9) + vh(1);
        const contentH = imgH + vh(1) + (state.items.length * itemH);
        const scrollH = vh(50);
        const maxScroll = Math.max(0.0, contentH - scrollH);

        const newY = Math.max(0.0, Math.min(state.scrollY + e.dy, maxScroll));
        SnifferUI.setState({ scrollY: newY });
        return "[]";
    }

    // Force update on window resize to refresh vw/vh/vmin
    if (e.type === "WindowResized") {
        SnifferUI.forceUpdate();
        return "[]";
    }

    return "[]";
};

// ---------------------------------------------------------------------------
// 4. START
// ---------------------------------------------------------------------------
host_create_image("test_img", ".plugins/default_ui/test.jpg");
SnifferUI.start(App, state);
