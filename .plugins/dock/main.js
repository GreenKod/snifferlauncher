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
    refreshEssentialApps();
    return getDockContainer(isLandscape);
});

registerApi("dock.handleClick", function(payload) {
    const id = payload && payload.id;
    if (id && dockState.hashMap && dockState.hashMap[id]) {
        const pkg = dockState.hashMap[id];
        launchApp(pkg);
        return { success: true, pkg: pkg };
    }
    return { success: false };
});

// Notify readiness on initialization
broadcastEvent("dock.ready", { ready: true });

// 3. Event Listener
globalThis.onEvent = function(eventJsonString) {
    const e = typeof eventJsonString === "string" ? JSON.parse(eventJsonString) : eventJsonString;

    if (e.type === "Click") {
        const idStr = String(e.id || "");
        if (!dockState.hashMap) {
            refreshEssentialApps();
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
