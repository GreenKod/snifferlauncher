// Default Interface - Main Entry Point

function App() {
    return ScrollView("root_scroll", {
        scroll_x: state.scrollX ?? 0.0,
        scroll_y: state.scrollY ?? 0.0,
        style: {
            display: "Flex",
            flex_direction: "Column",
            width: pct(100),
            height: pct(100),
            background_color: hex(state.bgColor),
            align_items: "Center",
            padding: padXY(vh(8.0), vw(5.0)),
            gap: vh(4.0),
        }
    }, [
        HeaderComponent(),
        SearchBarComponent(),
        ActionsGridComponent(),
        ...ModuleListComponent(),
    ]);
}

function Root() {
    return Container("root", {
        width: pct(100),
        height: pct(100),
        position: "Relative",
        background_color: hex(state.bgColor),
    }, [
        App(),
        FabButtonComponent(),
    ]);
}

subscribeChannel("clock.secondChanged", function(eventData) {
    host_log("[Default UI] Received clock.secondChanged event: " + JSON.stringify(eventData));
});

// Event Listener
globalThis.onEvent = function (eventJsonString) {
    const e = JSON.parse(eventJsonString);

    if (e.type === "Click" && e.id === host_hash("btn-greeting")) {
        SnifferUI.setState({
            greetingText: "Ready for Action! 🚀",
            bgColor: "#111111",
        });
        const time = callApi("clock.getTime", { "utcOffset": 3 });
        host_log("[Default UI] Current Time from Clock Widget: " + JSON.stringify(time));
        return "[]";
    }

    if (e.type === "Click" && (e.id === host_hash("counter_box") || e.id === host_hash("btn-counter") || e.id === host_hash("counter_val"))) {
        SnifferUI.setState({ clickCount: state.clickCount + 1 });
        return "[]";
    }

    if (e.type === "Click" && (e.id === host_hash("anim_box") || e.id === host_hash("anim_box_text"))) {
        SnifferUI.setState({ boxToggled: !state.boxToggled });
        return "[]";
    }

    if (e.type === "Click" && e.id === host_hash("search_input")) {
        host_focus_input("search_input");
        const query = state.searchQuery === "" ? "" : state.searchQuery;
        SnifferUI.setState({ isSearchFocused: true, searchQuery: query });
        return "[]";
    }

    if (e.type === "PointerDown" &&
        (String(e.id) === String(host_hash("fab_button")) ||
            String(e.id) === String(host_hash("fab_text")))) {
        SnifferUI.setState({ fabOpacity: 0.5 });
        return "[]";
    }

    if (e.type === "PointerUp") {
        const wasFabPressed = state.fabOpacity === 0.5;
        if (wasFabPressed) {
            const newItems = [...state.items, state.items.length + 1];
            SnifferUI.setState({ fabOpacity: 0.9, items: newItems });
        } else {
            SnifferUI.setState({ fabOpacity: 0.9 });
        }
        return "[]";
    }

    if (e.type === "ClickOutside") {
        host_blur_input();
        SnifferUI.setState({ isSearchFocused: false });
        return "[]";
    }

    if (e.type === "TextInput") {
        SnifferUI.setState({ searchQuery: state.searchQuery + e.text });
        return "[]";
    }

    if (e.type === "Backspace") {
        const val = state.searchQuery;
        if (val.length > 0) {
            SnifferUI.setState({ searchQuery: val.slice(0, -1) });
        }
        return "[]";
    }

    if (e.type === "Scroll" && e.id === host_hash("root_scroll")) {
        const s_x = (state.scrollX ?? 0.0) + e.dx;
        const s_y = (state.scrollY ?? 0.0) + e.dy;
        const max_x = e.max_x ?? 999999;
        const max_y = e.max_y ?? 999999;
        SnifferUI.setState({
            scrollX: Math.max(0, Math.min(s_x, max_x)),
            scrollY: Math.max(0, Math.min(s_y, max_y))
        });
        return "[]";
    }

    if (e.type === "WindowResized") {
        SnifferUI.forceUpdate();
        return "[]";
    }

    return "[]";
};

// Start the SnifferUI framework with our root component and initial state
SnifferUI.start(Root, state);
