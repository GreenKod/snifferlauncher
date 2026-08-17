// =============================================================================
// Sniffer Launcher - Modern Responsive Android Dock Plugin (Clean Quick Apps)
// =============================================================================

host_log("[Dock Plugin] Initializing Clean Android Dock plugin...");

// 1. İzinleri Talep Et (INPUT, IPC, UI)
const granted = requestPermissions([
    "plugin.permission.INPUT",
    "plugin.permission.IPC",
    "plugin.permission.UI"
]);

// 2. Temel Sistem Uygulamaları (Telefon, Mesajlar, Kamera, Ayarlar)
const ESSENTIAL_APPS_CONFIG = [
    { id: "phone", name: "Telefon", iconLetter: "T", color: "#22C55E", keywords: ["dialer", "phone", "telefon", "contacts", "rehber"] },
    { id: "messages", name: "Mesajlar", iconLetter: "M", color: "#3B82F6", keywords: ["messaging", "mms", "mesaj", "message", "sms", "chat"] },
    { id: "camera", name: "Kamera", iconLetter: "K", color: "#EC4899", keywords: ["camera", "kamera", "camera2"] },
    { id: "settings", name: "Ayarlar", iconLetter: "A", color: "#F59E0B", keywords: ["settings", "ayar", "setting"] }
];

// Dock State
const dockState = {
    essentialApps: [],
    detectedPackages: {}
};

/**
 * DataVault üzerinden sistemde yüklü olan temel uygulamaları eşleştirir.
 */
function refreshEssentialApps() {
    if (typeof Vault === "undefined" || typeof Vault.queryApps !== "function") {
        return;
    }

    try {
        const queryResult = Vault.queryApps({ limit: 100 });
        const allApps = queryResult.apps || [];
        const matched = [];
        const detected = {};

        for (const config of ESSENTIAL_APPS_CONFIG) {
            let foundApp = null;

            for (const app of allApps) {
                const pkgLower = (app.package_name || "").toLowerCase();
                const nameLower = (app.name || "").toLowerCase();

                const isMatch = config.keywords.some(function(kw) {
                    return pkgLower.includes(kw) || nameLower.includes(kw);
                });

                if (isMatch) {
                    foundApp = app;
                    break;
                }
            }

            if (foundApp) {
                matched.push({
                    id: config.id,
                    name: foundApp.name || config.name,
                    package_name: foundApp.package_name,
                    iconLetter: config.iconLetter,
                    color: config.color,
                    icon_src: foundApp.icon_src
                });
                detected[config.id] = foundApp.package_name;
            } else {
                matched.push({
                    id: config.id,
                    name: config.name,
                    package_name: null,
                    iconLetter: config.iconLetter,
                    color: config.color,
                    icon_src: null
                });
            }
        }

        dockState.essentialApps = matched;
        dockState.detectedPackages = detected;

        // Click için ID ve Hash tablosu oluştur
        const map = {};
        for (let i = 0; i < matched.length; i++) {
            const app = matched[i];
            if (!app.package_name) continue;
            const btnId = "dock_app_btn_" + (app.package_name || app.id);
            const circleBoxId = "dock_app_circle_" + app.id;
            const imgId = "dock_img_" + app.id;
            const circleId = "dock_circle_" + app.id;
            const iconId = "dock_app_icon_" + app.id;

            if (typeof host_hash === "function") {
                map[String(host_hash(btnId))] = app.package_name;
                map[String(host_hash(circleBoxId))] = app.package_name;
                map[String(host_hash(imgId))] = app.package_name;
                map[String(host_hash(circleId))] = app.package_name;
                map[String(host_hash(iconId))] = app.package_name;
            }
            map[btnId] = app.package_name;
            map[circleBoxId] = app.package_name;
            map[imgId] = app.package_name;
            map[circleId] = app.package_name;
            map[iconId] = app.package_name;
        }
        dockState.hashMap = map;
    } catch (e) {
        host_log("[Dock Plugin] Error matching apps: " + e);
    }
}

// Başlangıçta eşle
refreshEssentialApps();

// Sistem uygulamaları güncellendiğinde dock'u otomatik güncelle
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

// 3. Eklentiler Arası API Kayıtları (Inter-Plugin APIs)
registerApi("dock.getUI", function(payload) {
    const isLandscape = payload && payload.isLandscape;
    refreshEssentialApps();
    return getDockContainer(isLandscape);
});

registerApi("dock.handleClick", function(payload) {
    const id = payload && payload.id;
    if (id && dockState.hashMap[id]) {
        const pkg = dockState.hashMap[id];
        launchApp(pkg);
        return { success: true, pkg: pkg };
    }
    return { success: false };
});

// Başlangıçta hazır olduğunu bildir
broadcastEvent("dock.ready", { ready: true });

// 4. UI Bileşen Üretimi (Virtual DOM Ağacı)
function getDockContainer(isLandscape) {
    if (isLandscape) {
        // Yatay (Landscape) Mod: Sağda Dikey Dock
        return Container("dock_root_container", {
            width: px(vw(13.0)),
            height: pct(100),
            background_color: hex("#00000000"),
            flex_direction: "Column",
            justify_content: "Center",
            align_items: "Center",
            padding: padXY(vmin(1.0), vmin(2.0)),
            gap: vh(2.8),
        }, dockState.essentialApps.map(function(app) {
            const iconSize = vw(8.5);
            const iconRadius = iconSize / 2.0;
            const touchW = vw(13.0);
            const touchH = vw(13.0);
            
            return Container("dock_app_btn_" + (app.package_name || app.id), {
                width: px(touchW),
                height: px(touchH),
                background_color: hex("#00000000"),
                justify_content: "Center",
                align_items: "Center",
            }, [
                Container("dock_app_circle_" + app.id, {
                    width: px(iconSize),
                    height: px(iconSize),
                    background_color: hex("#1A223499"),
                    border_radius: iconRadius,
                    border_width: px(1.0),
                    border_color: hex("#33415566"),
                    justify_content: "Center",
                    align_items: "Center",
                }, [
                    app.package_name ? Image("dock_img_" + app.id, "app-icon://" + app.package_name, {
                        width: px(iconSize * 0.84),
                        height: px(iconSize * 0.84),
                        border_radius: (iconSize * 0.84) / 2.0,
                        object_fit: "Cover"
                    }) : Container("dock_circle_" + app.id, {
                        width: px(iconSize * 0.84),
                        height: px(iconSize * 0.84),
                        border_radius: (iconSize * 0.84) / 2.0,
                        background_color: hex(app.color),
                        justify_content: "Center",
                        align_items: "Center",
                    }, [
                        Label("dock_app_icon_" + app.id, app.iconLetter, {
                            text_color: hex("#FFFFFF"),
                            text_size: vw(2.2),
                            width: "Auto",
                        })
                    ])
                ])
            ]);
        }));
    }

    // Dikey (Portrait) Mod: Altta Yüzen Kusursuz Şeffaf Android Dock (Dengeli Ergonomik Hitbox)
    return Container("dock_root_container", {
        width: pct(100),
        height: px(vh(18.0)),
        background_color: hex("#00000000"),
        flex_direction: "Column",
        justify_content: "Center",
        align_items: "Center",
        padding: {
            top: 0,
            bottom: vmin(5.0),
            left: vmin(2.0),
            right: vmin(2.0)
        },
    }, [
        Container("dock_apps_row_port", {
            width: pct(92),
            height: px(vmin(17.5)),
            flex_direction: "Row",
            justify_content: "SpaceAround",
            align_items: "Center",
        }, dockState.essentialApps.map(function(app) {
            const visualSize = vmin(15.0);
            const visualRadius = visualSize / 2.0;
            const touchW = vmin(17.0);
            const touchH = vmin(17.5);

            return Container("dock_app_btn_" + (app.package_name || app.id), {
                width: px(touchW),
                height: px(touchH),
                background_color: hex("#00000000"),
                justify_content: "Center",
                align_items: "Center",
            }, [
                Container("dock_app_circle_" + app.id, {
                    width: px(visualSize),
                    height: px(visualSize),
                    background_color: hex("#1A223499"),
                    border_radius: visualRadius,
                    border_width: px(1.0),
                    border_color: hex("#33415566"),
                    justify_content: "Center",
                    align_items: "Center",
                }, [
                    app.package_name ? Image("dock_img_" + app.id, "app-icon://" + app.package_name, {
                        width: px(visualSize * 0.90),
                        height: px(visualSize * 0.90),
                        border_radius: (visualSize * 0.90) / 2.0,
                        object_fit: "Cover"
                    }) : Container("dock_circle_" + app.id, {
                        width: px(visualSize * 0.90),
                        height: px(visualSize * 0.90),
                        border_radius: (visualSize * 0.90) / 2.0,
                        background_color: hex(app.color),
                        justify_content: "Center",
                        align_items: "Center",
                    }, [
                        Label("dock_app_icon_" + app.id, app.iconLetter, {
                            text_color: hex("#FFFFFF"),
                            text_size: vmin(5.6),
                            width: "Auto",
                        })
                    ])
                ])
            ]);
        }))
    ]);
}

// 5. Olay Dinleyicisi
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
