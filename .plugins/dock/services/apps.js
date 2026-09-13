// Dock State & Essential Apps Matching Service
const ESSENTIAL_APPS_CONFIG = [
    { id: "phone", name: "Telefon", iconLetter: "T", color: "#22C55E", keywords: ["dialer", "phone", "telefon", "contacts", "rehber"] },
    { id: "messages", name: "Mesajlar", iconLetter: "M", color: "#3B82F6", keywords: ["messaging", "mms", "mesaj", "message", "sms", "chat"] },
    { id: "camera", name: "Kamera", iconLetter: "K", color: "#EC4899", keywords: ["camera", "kamera", "camera2"] },
    { id: "settings", name: "Ayarlar", iconLetter: "A", color: "#F59E0B", keywords: ["settings", "ayar", "setting"] }
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
    hashMap: {}
};

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
        return;
    }

    try {
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

        // Build ID and Hash map for click hit testing
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
