// =============================================================================
// Sniffer Launcher - Modern Responsive Android Dock Plugin (Clean Quick Apps)
// =============================================================================

host_log("[Dock Plugin] Initializing Clean Android Dock plugin...");

// 1. Request Permissions (INPUT, IPC, UI)
const granted = requestPermissions([
    "plugin.permission.INPUT",
    "plugin.permission.IPC",
    "plugin.permission.UI"
]);

// Initial match
refreshEssentialApps();

// Automatically update dock when system applications change
if (typeof subscribeChannel === "function") {
    subscribeChannel("vault.changed:system.apps", function() {
        refreshEssentialApps();
        if (typeof broadcastEvent === "function") {
            broadcastEvent("dock.stateChanged", {});
        }
    });
    subscribeChannel("system.apps", function() {
        refreshEssentialApps();
        if (typeof broadcastEvent === "function") {
            broadcastEvent("dock.stateChanged", {});
        }
    });
} else if (typeof Vault !== "undefined" && typeof Vault.subscribe === "function") {
    Vault.subscribe("system.apps", function() {
        refreshEssentialApps();
        if (typeof broadcastEvent === "function") {
            broadcastEvent("dock.stateChanged", {});
        }
    });
}

// 2. Inter-Plugin API Registrations
registerApi("dock.getUI", function(payload) {
    const isLandscape = payload && payload.isLandscape;
    return getDockContainer(isLandscape);
});

registerApi("dock.handleClick", function(payload) {
    const id = payload && payload.id;
    if (id) {
        const idStr = String(id);
        const appId = dockState.appMap ? dockState.appMap[idStr] : null;
        if (appId) {
            dockState.pressedAppId = appId;
            if (typeof broadcastEvent === "function") {
                broadcastEvent("dock.stateChanged", { pressedAppId: appId });
            }
            if (typeof setTimeout === "function") {
                setTimeout(function() {
                    if (dockState.pressedAppId === appId) {
                        dockState.pressedAppId = null;
                        if (typeof broadcastEvent === "function") {
                            broadcastEvent("dock.stateChanged", { pressedAppId: null });
                        }
                    }
                }, 180);
            } else {
                dockState.pressedAppId = null;
            }
        }
    }
    if (id && dockState.hashMap && dockState.hashMap[id]) {
        const pkg = dockState.hashMap[id];
        launchApp(pkg);
        return { success: true, pkg: pkg };
    }
    return { success: false };
});

registerApi("dock.setPressed", function(payload) {
    const id = payload && payload.id;
    const pressed = payload && payload.pressed;
    if (id && dockState.appMap) {
        const appId = dockState.appMap[String(id)] || id;
        dockState.pressedAppId = pressed ? appId : null;
        if (typeof broadcastEvent === "function") {
            broadcastEvent("dock.stateChanged", { pressedAppId: dockState.pressedAppId });
        }
        return { success: true, pressedAppId: dockState.pressedAppId };
    }
    return { success: false };
});

// Notify readiness on initialization
broadcastEvent("dock.ready", { ready: true });

// 3. Event Listener
globalThis.onEvent = function(eventJsonString) {
    const e = typeof eventJsonString === "string" ? JSON.parse(eventJsonString) : eventJsonString;

    if (e.type === "PointerDown") {
        const idStr = String(e.id || "");
        const appId = dockState.appMap ? dockState.appMap[idStr] : null;
        if (appId) {
            dockState.pressedAppId = appId;
            if (typeof broadcastEvent === "function") {
                broadcastEvent("dock.stateChanged", { pressedAppId: appId });
            }
        }
    } else if (e.type === "PointerUp") {
        if (dockState.pressedAppId) {
            dockState.pressedAppId = null;
            if (typeof broadcastEvent === "function") {
                broadcastEvent("dock.stateChanged", { pressedAppId: null });
            }
        }
    } else if (e.type === "Click") {
        const idStr = String(e.id || "");
        const appId = dockState.appMap ? dockState.appMap[idStr] : null;
        if (appId) {
            dockState.pressedAppId = appId;
            if (typeof broadcastEvent === "function") {
                broadcastEvent("dock.stateChanged", { pressedAppId: appId });
            }
            if (typeof setTimeout === "function") {
                setTimeout(function() {
                    if (dockState.pressedAppId === appId) {
                        dockState.pressedAppId = null;
                        if (typeof broadcastEvent === "function") {
                            broadcastEvent("dock.stateChanged", { pressedAppId: null });
                        }
                    }
                }, 180);
            } else {
                dockState.pressedAppId = null;
            }
        }

        const pkg = dockState.hashMap ? dockState.hashMap[idStr] : null;
        if (pkg) {
            host_log("[Dock Plugin] Launching essential app from onEvent: " + pkg);
            if (typeof launchApp === "function") {
                launchApp(pkg);
            }
            return "[]";
        }
    }

    return "[]";
};

