// Dock State & Essential Apps Matching Service
const ESSENTIAL_APPS_CONFIG = [
    {
        id: "phone",
        name: "Telefon",
        iconLetter: "T",
        color: "#22C55E",
        preferredPackages: [
            "com.sh.smart.caller",
            "com.google.android.dialer",
            "com.samsung.android.dialer",
            "com.android.dialer"
        ],
        keywords: ["dialer", "phone", "telefon", "caller", "contacts", "rehber"]
    },
    {
        id: "messages",
        name: "Mesajlar",
        iconLetter: "M",
        color: "#3B82F6",
        preferredPackages: [
            "com.google.android.apps.messaging",
            "com.transsion.smartmessage",
            "com.samsung.android.messaging",
            "com.android.mms"
        ],
        keywords: ["messaging", "mms", "mesaj", "message", "sms"],
        excludeKeywords: ["chatgpt", "deepseek", "kimi", "openai", "copilot", "claude"]
    },
    {
        id: "camera",
        name: "Kamera",
        iconLetter: "K",
        color: "#EC4899",
        preferredPackages: [
            "com.transsion.camera",
            "com.google.android.GoogleCamera",
            "com.sec.android.app.camera",
            "com.android.camera"
        ],
        keywords: ["camera", "kamera", "camera2"]
    },
    {
        id: "settings",
        name: "Ayarlar",
        iconLetter: "A",
        color: "#F59E0B",
        preferredPackages: [
            "com.android.settings"
        ],
        keywords: ["settings", "ayar", "setting"]
    }
];

const DEFAULT_ESSENTIAL_APPS = ESSENTIAL_APPS_CONFIG.map(function(cfg) {
    return {
        id: cfg.id,
        name: cfg.name,
        package_name: null,
        iconLetter: cfg.iconLetter,
        color: cfg.color,
        icon_src: null
    };
});

const dockState = {
    essentialApps: DEFAULT_ESSENTIAL_APPS.slice(),
    detectedPackages: {},
    hashMap: {},
    appMap: {},
    pressedAppId: null,
    isLandscape: false,
    badges: {},
    activePreviewAppId: null,
    longPressTimer: null
};

function rebuildDockIdMaps() {
    const map = {};
    const appMap = {};
    const apps = dockState.essentialApps || [];

    for (let i = 0; i < apps.length; i++) {
        const app = apps[i];
        const appId = app.id;
        const btnId = "dock_app_btn_" + (app.package_name || app.id);
        const circleBoxId = "dock_app_circle_" + app.id;
        const imgId = "dock_img_" + app.id;
        const circleId = "dock_circle_" + app.id;
        const iconId = "dock_app_icon_" + app.id;
        const badgeCounterId = "dock_badge_counter_" + app.id;
        const badgeDotId = "dock_badge_dot_" + app.id;

        if (typeof host_hash === "function") {
            if (app.package_name) {
                map[String(host_hash(btnId))] = app.package_name;
                map[String(host_hash(circleBoxId))] = app.package_name;
                map[String(host_hash(imgId))] = app.package_name;
                map[String(host_hash(circleId))] = app.package_name;
                map[String(host_hash(iconId))] = app.package_name;
                map[String(host_hash(badgeCounterId))] = app.package_name;
                map[String(host_hash(badgeDotId))] = app.package_name;
            }
            appMap[String(host_hash(btnId))] = appId;
            appMap[String(host_hash(circleBoxId))] = appId;
            appMap[String(host_hash(imgId))] = appId;
            appMap[String(host_hash(circleId))] = appId;
            appMap[String(host_hash(iconId))] = appId;
            appMap[String(host_hash(badgeCounterId))] = appId;
            appMap[String(host_hash(badgeDotId))] = appId;
        }

        if (app.package_name) {
            map[btnId] = app.package_name;
            map[circleBoxId] = app.package_name;
            map[imgId] = app.package_name;
            map[circleId] = app.package_name;
            map[iconId] = app.package_name;
            map[badgeCounterId] = app.package_name;
            map[badgeDotId] = app.package_name;
        }
        appMap[btnId] = appId;
        appMap[circleBoxId] = appId;
        appMap[imgId] = appId;
        appMap[circleId] = appId;
        appMap[iconId] = appId;
        appMap[badgeCounterId] = appId;
        appMap[badgeDotId] = appId;
    }

    dockState.hashMap = map;
    dockState.appMap = appMap;
}

// Initial ID map build
rebuildDockIdMaps();

function refreshEssentialApps() {
    let allApps = [];
    try {
        if (typeof Vault !== "undefined" && typeof Vault.queryApps === "function") {
            const queryResult = Vault.queryApps({ limit: 100 });
            allApps = (queryResult && queryResult.apps) ? queryResult.apps : [];
        }
        if (allApps.length === 0 && typeof getApplicationList === "function") {
            allApps = getApplicationList() || [];
        }
    } catch (e) {
        allApps = [];
    }

    if (allApps.length === 0) {
        rebuildDockIdMaps();
        return;
    }

    try {
        const matched = [];
        const detected = {};

        for (const config of ESSENTIAL_APPS_CONFIG) {
            let foundApp = null;

            // 1. Try preferred package match first
            if (config.preferredPackages) {
                for (const prefPkg of config.preferredPackages) {
                    const match = allApps.find(function(a) {
                        return (a.package_name || "").toLowerCase() === prefPkg.toLowerCase();
                    });
                    if (match) {
                        foundApp = match;
                        break;
                    }
                }
            }

            // 2. Fall back to keywords
            if (!foundApp) {
                for (const app of allApps) {
                    const pkgLower = (app.package_name || "").toLowerCase();
                    const nameLower = (app.name || "").toLowerCase();

                    if (config.excludeKeywords && config.excludeKeywords.some(function(ex) {
                        return pkgLower.includes(ex) || nameLower.includes(ex);
                    })) {
                        continue;
                    }

                    const isMatch = config.keywords.some(function(kw) {
                        return pkgLower.includes(kw) || nameLower.includes(kw);
                    });

                    if (isMatch) {
                        foundApp = app;
                        break;
                    }
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

        rebuildDockIdMaps();
    } catch (e) {
        host_log("[Dock Plugin] Error matching apps: " + e);
    }
}

