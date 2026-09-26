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
    subscribeChannel("system.notifications", function(payload) {
        if (payload && payload.package_name && dockState.essentialApps) {
            const pkg = payload.package_name;
            const app = dockState.essentialApps.find(function(a) { return a.package_name === pkg; });
            if (app) {
                if (payload.count > 0 || payload.dot) {
                    dockState.badges[app.id] = {
                        count: payload.count || 0,
                        dot: !!payload.dot
                    };
                } else {
                    delete dockState.badges[app.id];
                }
                if (typeof broadcastEvent === "function") {
                    broadcastEvent("dock.badgeChanged", { id: app.id, badge: dockState.badges[app.id] || null });
                    broadcastEvent("dock.stateChanged", {});
                }
            }
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
    const isLandscape = Boolean(payload && payload.isLandscape);
    dockState.isLandscape = isLandscape;
    return getDockContainer(isLandscape);
});

registerApi("dock.setOrientation", function(payload) {
    const isLandscape = Boolean(payload && payload.isLandscape);
    if (dockState.isLandscape !== isLandscape) {
        dockState.isLandscape = isLandscape;
        if (typeof broadcastEvent === "function") {
            broadcastEvent("dock.orientationChanged", { isLandscape: isLandscape });
            broadcastEvent("dock.stateChanged", { isLandscape: isLandscape });
        }
    }
    return { success: true, isLandscape: dockState.isLandscape };
});

registerApi("dock.setBadge", function(payload) {
    const id = payload && payload.id;
    if (id) {
        const idStr = String(id);
        const appId = (dockState.appMap && dockState.appMap[idStr]) || idStr;
        if (payload.count === 0 && !payload.dot) {
            delete dockState.badges[appId];
        } else {
            dockState.badges[appId] = {
                count: typeof payload.count === "number" ? payload.count : 0,
                dot: Boolean(payload.dot || (payload.count === undefined && payload.dot !== false))
            };
        }
        if (typeof broadcastEvent === "function") {
            broadcastEvent("dock.badgeChanged", { id: appId, badge: dockState.badges[appId] || null });
            broadcastEvent("dock.stateChanged", {});
        }
        return { success: true, badge: dockState.badges[appId] || null };
    }
    return { success: false };
});

registerApi("dock.clearBadge", function(payload) {
    const id = payload && payload.id;
    if (id) {
        const idStr = String(id);
        const appId = (dockState.appMap && dockState.appMap[idStr]) || idStr;
        delete dockState.badges[appId];
        if (typeof broadcastEvent === "function") {
            broadcastEvent("dock.badgeChanged", { id: appId, badge: null });
            broadcastEvent("dock.stateChanged", {});
        }
        return { success: true };
    }
    return { success: false };
});

registerApi("dock.triggerLongPress", function(payload) {
    const id = payload && payload.id;
    if (id) {
        const idStr = String(id);
        const appId = (dockState.appMap && dockState.appMap[idStr]) || idStr;
        dockState.activePreviewAppId = appId;
        dockState.pressedAppId = null;
        if (typeof broadcastEvent === "function") {
            broadcastEvent("dock.longPress", { id: appId });
            broadcastEvent("dock.stateChanged", { activePreviewAppId: appId });
        }
        return { success: true, activePreviewAppId: appId };
    }
    return { success: false };
});

registerApi("dock.dismissPreview", function() {
    if (dockState.activePreviewAppId) {
        dockState.activePreviewAppId = null;
        if (typeof broadcastEvent === "function") {
            broadcastEvent("dock.previewDismissed", {});
            broadcastEvent("dock.stateChanged", { activePreviewAppId: null });
        }
        return { success: true };
    }
    return { success: false };
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

    if (e.type === "OrientationChange" || e.type === "Resize") {
        const isLandscape = Boolean(e.isLandscape !== undefined ? e.isLandscape : (e.width && e.height && e.width > e.height));
        if (dockState.isLandscape !== isLandscape) {
            dockState.isLandscape = isLandscape;
            if (typeof broadcastEvent === "function") {
                broadcastEvent("dock.orientationChanged", { isLandscape: isLandscape });
                broadcastEvent("dock.stateChanged", { isLandscape: isLandscape });
            }
        }
    } else if (e.type === "PointerDown") {
        if (dockState.longPressTimer) {
            clearTimeout(dockState.longPressTimer);
            dockState.longPressTimer = null;
        }

        const idStr = String(e.id || "");
        const appId = dockState.appMap ? dockState.appMap[idStr] : null;
        if (appId) {
            dockState.pressedAppId = appId;
            if (typeof broadcastEvent === "function") {
                broadcastEvent("dock.stateChanged", { pressedAppId: appId });
            }
            if (typeof setTimeout === "function") {
                dockState.longPressTimer = setTimeout(function() {
                    dockState.longPressTimer = null;
                    if (dockState.pressedAppId === appId) {
                        dockState.pressedAppId = null;
                        dockState.activePreviewAppId = appId;
                        if (typeof broadcastEvent === "function") {
                            broadcastEvent("dock.longPress", { id: appId });
                            broadcastEvent("dock.stateChanged", { activePreviewAppId: appId });
                        }
                    }
                }, 400);
            }
        }
    } else if (e.type === "PointerUp") {
        if (dockState.longPressTimer) {
            clearTimeout(dockState.longPressTimer);
            dockState.longPressTimer = null;
        }
        if (dockState.pressedAppId) {
            dockState.pressedAppId = null;
            if (typeof broadcastEvent === "function") {
                broadcastEvent("dock.stateChanged", { pressedAppId: null });
            }
        }
    } else if (e.type === "Click") {
        if (dockState.longPressTimer) {
            clearTimeout(dockState.longPressTimer);
            dockState.longPressTimer = null;
        }

        const idStr = String(e.id || "");

        // Handle preview close button click
        if (idStr === "dock_preview_close_btn" || idStr === "dock_preview_close_txt") {
            dockState.activePreviewAppId = null;
            if (typeof broadcastEvent === "function") {
                broadcastEvent("dock.stateChanged", { activePreviewAppId: null });
            }
            return "[]";
        }

        // Handle preview bubble card click to launch
        if (idStr === "dock_preview_bubble" || idStr === "dock_preview_title" || idStr === "dock_preview_action") {
            if (dockState.activePreviewAppId) {
                const previewApp = dockState.essentialApps.find(function(a) { return a.id === dockState.activePreviewAppId; });
                dockState.activePreviewAppId = null;
                if (previewApp && previewApp.package_name && typeof launchApp === "function") {
                    launchApp(previewApp.package_name);
                }
                if (typeof broadcastEvent === "function") {
                    broadcastEvent("dock.stateChanged", { activePreviewAppId: null });
                }
            }
            return "[]";
        }

        // Dismiss preview if clicking anywhere else
        if (dockState.activePreviewAppId) {
            dockState.activePreviewAppId = null;
            if (typeof broadcastEvent === "function") {
                broadcastEvent("dock.stateChanged", { activePreviewAppId: null });
            }
        }

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
