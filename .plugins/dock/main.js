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
// Font glif uyumluluğu için temiz harf ikonları kullanıldı (? hatasını önler)
const ESSENTIAL_APPS_CONFIG = [
    { id: "phone", name: "Telefon", iconLetter: "T", color: "#22C55E", keywords: ["dialer", "phone", "telefon", "contacts", "rehber"] },
    { id: "messages", name: "Mesajlar", iconLetter: "M", color: "#3B82F6", keywords: ["messaging", "mms", "mesaj", "message", "sms"] },
    { id: "camera", name: "Kamera", iconLetter: "K", color: "#EC4899", keywords: ["camera", "kamera"] },
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
                    package_name: "com.android." + config.id,
                    iconLetter: config.iconLetter,
                    color: config.color,
                    icon_src: null
                });
            }
        }

        dockState.essentialApps = matched;
        dockState.detectedPackages = detected;
    } catch (e) {
        host_log("[Dock Plugin] Error matching apps: " + e);
    }
}

// Başlangıçta eşle
refreshEssentialApps();

// Sistem uygulamaları güncellendiğinde dock'u otomatik güncelle
if (typeof Vault !== "undefined" && typeof Vault.subscribe === "function") {
    Vault.subscribe("system.apps", function() {
        refreshEssentialApps();
        broadcastEvent("dock.stateChanged", {});
    });
}

// 3. Eklentiler Arası API Kayıtları (Inter-Plugin APIs)
registerApi("dock.getUI", function(payload) {
    const isLandscape = payload && payload.isLandscape;
    return renderDockUI(isLandscape);
});

// Başlangıçta hazır olduğunu bildir
broadcastEvent("dock.ready", { ready: true });

// 4. Modern Yüzen Android Dock Render
function renderDockUI(isLandscape) {
    // Yatay (Landscape) Mod: Sağda İnce & Şık Dikey Dock
    if (isLandscape) {
        return Container("dock_root_container", {
            width: px(vw(13.0)),
            height: pct(100),
            background_color: hex("#0C101BE6"),
            border_left_width: px(1.0),
            border_left_color: hex("#1E293B88"),
            flex_direction: "Column",
            justify_content: "Center",
            align_items: "Center",
            padding: padXY(vmin(1.0), vmin(2.0)),
            gap: vh(2.8),
        }, dockState.essentialApps.map(function(app) {
            const btnSize = vw(8.5);
            return Container("dock_app_btn_" + app.package_name, {
                width: px(btnSize),
                height: px(btnSize),
                background_color: hex("#1A2234"),
                border_radius: btnSize / 2.0,
                border_width: px(1.0),
                border_color: hex("#334155AA"),
                flex_direction: "Column",
                justify_content: "Center",
                align_items: "Center",
            }, [
                Container("dock_circle_" + app.id, {
                    width: px(btnSize * 0.76),
                    height: px(btnSize * 0.76),
                    border_radius: (btnSize * 0.76) / 2.0,
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
            ]);
        }));
    }

    // Dikey (Portrait) Mod: Altta Yüzen Modern Android Dock
    return Container("dock_root_container", {
        width: pct(100),
        height: px(vh(11.0)),
        background_color: hex("#0C101BE6"),
        border_top_width: px(1.0),
        border_top_color: hex("#1E293B88"),
        flex_direction: "Column",
        justify_content: "Center",
        align_items: "Center",
        padding: padXY(vmin(3.0), vmin(1.0)),
        gap: vh(0.8),
    }, [
        Container("dock_apps_row_port", {
            width: pct(92),
            height: px(vh(8.2)),
            flex_direction: "Row",
            justify_content: "SpaceAround",
            align_items: "Center",
        }, dockState.essentialApps.map(function(app) {
            const btnSize = vmin(14.0);
            return Container("dock_app_btn_" + app.package_name, {
                width: px(btnSize),
                height: px(btnSize),
                background_color: hex("#1A2234"),
                border_radius: btnSize / 2.0,
                border_width: px(1.0),
                border_color: hex("#334155AA"),
                justify_content: "Center",
                align_items: "Center",
            }, [
                Container("dock_circle_" + app.id, {
                    width: px(btnSize * 0.78),
                    height: px(btnSize * 0.78),
                    border_radius: (btnSize * 0.78) / 2.0,
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
            ]);
        })),

        // Alt Home Göstergesi
        Container("dock_home_indicator", {
            width: px(vmin(28.0)),
            height: px(3.0),
            background_color: hex("#47556988"),
            border_radius: px(1.5),
        }, [])
    ]);
}

// 5. Olay Dinleyicisi
globalThis.onEvent = function(eventJsonString) {
    const e = typeof eventJsonString === "string" ? JSON.parse(eventJsonString) : eventJsonString;

    if (e.type === "Click") {
        const idStr = String(e.id || "");

        // Temel Uygulamayı Başlat
        if (idStr.startsWith("dock_app_btn_")) {
            const pkg = idStr.replace("dock_app_btn_", "");
            host_log("[Dock Plugin] Launching essential app: " + pkg);
            if (typeof launchApp === "function") {
                launchApp(pkg);
            }
            return "[]";
        }
    }

    return "[]";
};
