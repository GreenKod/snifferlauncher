// Default Interface - Main Entry Point
// Uses Rust scroll physics engine.
// JS does not need to manually track isDragging / dragOffset / kinetic scroll.

function getDockElement(isLandscape) {
    let dockUI = null;
    if (typeof callApi === "function") {
        try {
            dockUI = callApi("dock.getUI", { isLandscape: isLandscape });
        } catch (e) {
            dockUI = null;
        }
    }
    if (dockUI) {
        return dockUI;
    }

    // Fallback if dock plugin is still initializing
    return Container("dock_fallback_area", {
        width: isLandscape ? px(vw(13.0)) : pct(100),
        height: isLandscape ? pct(100) : px(vh(18.0)),
        justify_content: "Center",
        align_items: "Center",
    }, [
        Label("dock_fallback_label", "...", {
            text_color: hex("#666666"),
            text_size: vmin(2.5),
            width: "Auto",
        })
    ]);
}

function getDevKitHUD() {
    if (!state.showDevKitHud) {
        return null;
    }
    let devkitUI = null;
    if (typeof callApi === "function") {
        try {
            devkitUI = callApi("devkit_hud.getUI");
        } catch (e) {
            devkitUI = null;
        }
    }
    return devkitUI;
}

function getDrawerHint(_isLandscape) {
    return null;
}

function Root() {
    const isLandscape = vw(100) > vh(100);
    const hud = getDevKitHUD();
    const isBottomFloor = state.currentFloor === "bottom";

    // Floor 1: Home Screen Layer with smooth transition
    const hint = getDrawerHint(isLandscape);
    const homeChildren = [
        AppGridComponent(),
        getDockElement(isLandscape)
    ];

    if (hint) {
        homeChildren.push(hint);
    }

    const homeScreen = Container("home_screen_layer", {
        width: pct(100),
        height: pct(100),
        position: "Relative",
        background_color: state.bgColor ? hex(state.bgColor) : undefined,
        flex_direction: isLandscape ? "Row" : "Column",
        justify_content: "Start",
        align_items: "Stretch",
        opacity: isBottomFloor ? 0.0 : 1.0,
        transform: {
            scale: isBottomFloor ? 0.92 : 1.0,
            translate_y: isBottomFloor ? -vh(12.0) : 0.0,
        },
        transition: {
            duration: 0.28,
            easing: "ease_out"
        }
    }, homeChildren);

    const rootChildren = [homeScreen];

    // Floor 2: App Drawer Layer as animated overlay
    if (typeof AppDrawerComponent === "function") {
        rootChildren.push(AppDrawerComponent());
    }

    if (hud) {
        rootChildren.push(hud);
    }

    return Container("root", {
        width: pct(100),
        height: pct(100),
        position: "Relative",
    }, rootChildren);
}

// Inter-Plugin Communication (IPC): Listen for broadcasts from dock plugin
subscribeChannel("dock.ready", function() {
    SnifferUI.forceUpdate();
    if (typeof broadcastEvent === "function") {
        broadcastEvent("default_ui.stats", { appCount: state.allApps.length });
    }
});

subscribeChannel("devkit_hud.ready", function() {
    SnifferUI.forceUpdate();
});

subscribeChannel("dock.stateChanged", function() {
    SnifferUI.forceUpdate();
});

subscribeChannel("dock.textChanged", function(data) {
    if (data && typeof data.text === "string") {
        state.searchQuery = data.text;
        SnifferUI.forceUpdate();
        if (typeof broadcastEvent === "function") {
            broadcastEvent("default_ui.stats", { appCount: state.filteredApps.length });
        }
    }
});

// Native Data Vault Reactive Listener (instead of polling)
if (typeof Vault !== "undefined" && typeof Vault.subscribe === "function") {
    Vault.subscribe("system.apps", function() {
        state.refreshApps();
        SnifferUI.forceUpdate();
        if (typeof broadcastEvent === "function") {
            broadcastEvent("default_ui.stats", { appCount: state.allApps.length });
        }
    });
}

if (typeof broadcastEvent === "function") {
    broadcastEvent("default_ui.stats", { appCount: state.allApps.length });
}

let _hasInitialRefreshed = false;

// Invoked when Rust scroll physics completes a page snap.
// Used to update page indicator dots.
function onPageChanged(pageData) {
    const data = typeof pageData === "string" ? JSON.parse(pageData) : pageData;
    if (typeof data.page === "number") {
        state.currentPage = data.page;
        SnifferUI.forceUpdate();
    }
}

globalThis.onEvent = function (eventJsonString) {
    if (!_hasInitialRefreshed) {
        _hasInitialRefreshed = true;
        // Check if we still have mock apps before refreshing on first interaction
        if (state.allApps.length > 0 && state.allApps[0].id === "mock_app_1") {
            let realApps = getApplicationList();
            if (realApps.length > 0) {
                state.refreshApps();
                SnifferUI.forceUpdate();
            }
        }
    }

    const e = JSON.parse(eventJsonString);

    if (e.type === "WindowResized") {
        // Taffy layout engine calculates dynamic sizes on forceUpdate
        SnifferUI.forceUpdate();
        return "[]";
    }

    // PageSnapped: Rust physics engine notified snap completion
    if (e.type === "PageSnapped") {
        if (typeof onPageChanged === "function") {
            onPageChanged({ page: e.page });
        }
        return "[]";
    }

    // SwipeUp: Go down to bottom floor (App Drawer)
    if (e.type === "SwipeUp") {
        if (state.currentFloor !== "bottom") {
            state.goToBottomFloor();
        }
        return "[]";
    }

    // SwipeDown: Go up to top floor (Home Screen)
    if (e.type === "SwipeDown") {
        if (state.currentFloor === "bottom") {
            state.goToTopFloor();
        }
        return "[]";
    }

    if (e.type === "TextInput") {
        if (state.currentFloor === "bottom" && typeof e.text === "string") {
            state.drawerSearchQuery = (state.drawerSearchQuery || "") + e.text;
            state.rebuildCardHashCache();
            SnifferUI.forceUpdate();
        }
        return "[]";
    }

    if (e.type === "Backspace") {
        if (state.currentFloor === "bottom") {
            state.drawerSearchQuery = (state.drawerSearchQuery || "").slice(0, -1);
            state.rebuildCardHashCache();
            SnifferUI.forceUpdate();
        }
        return "[]";
    }

    if (e.type === "Click") {
        const idStr = String(e.id || "");

        // Close App Drawer -> Go to Top Floor
        if (
            idStr === "drawer_close_btn" ||
            idStr === "drawer_close_icon" ||
            idStr === "drawer_handle" ||
            idStr === "drawer_handle_pill_wrapper"
        ) {
            state.goToTopFloor();
            return "[]";
        }

        // Open App Drawer -> Go to Bottom Floor
        if (
            idStr === "open_drawer_hint" ||
            idStr === "drawer_hint_icon" ||
            idStr === "drawer_hint_text"
        ) {
            state.goToBottomFloor();
            return "[]";
        }

        // Clear Search Query in App Drawer
        if (idStr === "drawer_search_clear" || idStr === "drawer_clear_label") {
            state.drawerSearchQuery = "";
            state.rebuildCardHashCache();
            SnifferUI.forceUpdate();
            return "[]";
        }

        if (
            idStr === "drawer_search_input" ||
            idStr === "drawer_search_container" ||
            idStr === "drawer_search_left_group"
        ) {
            state.isSearchFocused = true;
            if (typeof focusInput === "function") {
                focusInput("drawer_search_input");
            }
            return "[]";
        }

        // App card click (both main app grid and app drawer)
        const pkg = state.getAppPackageByHash(idStr);
        if (typeof host_log === "function") {
            host_log("[SnifferLauncher JS] Click received for ID: " + idStr + " => Resolved Package: " + (pkg || "null"));
        }
        if (pkg) {
            launchApp(pkg);
        }
        return "[]";
    }

    // Scroll, PointerDown, PointerUp: handled by Rust engine
    return "[]";
};

if (typeof subscribeChannel === "function") {
    subscribeChannel("dock.stateChanged", function() {
        SnifferUI.forceUpdate();
    });
    subscribeChannel("dock.ready", function() {
        SnifferUI.forceUpdate();
    });
}

SnifferUI.start(Root, state);

if (typeof setInterval === "function") {
    setInterval(function() {
        if (getDevKitHUD() !== null) {
            SnifferUI.forceUpdate();
        }
    }, 500);
}
