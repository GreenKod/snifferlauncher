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

function Root() {
    const isLandscape = vw(100) > vh(100);
    const hud = getDevKitHUD();

    const rootChildren = [
        AppGridComponent(),
        getDockElement(isLandscape)
    ];

    if (hud) {
        rootChildren.push(hud);
    }

    if (isLandscape) {
        return Container("root", {
            width: pct(100),
            height: pct(100),
            position: "Relative",
            background_color: state.bgColor ? hex(state.bgColor) : undefined,
            flex_direction: "Row",
            justify_content: "Start",
            align_items: "Stretch",
        }, rootChildren);
    }

    return Container("root", {
        width: pct(100),
        height: pct(100),
        position: "Relative",
        background_color: state.bgColor ? hex(state.bgColor) : undefined,
        flex_direction: "Column",
        justify_content: "Start",
        align_items: "Stretch",
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

    if (e.type === "Click") {
        const idStr = String(e.id || "");

        // Main app grid click (dock clicks are handled by the dock plugin)
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
