// =============================================================================
// System & Native Application Bridge
// =============================================================================

globalThis.getApplicationList = function() {
    try {
        return JSON.parse(host_get_application_list());
    } catch(e) {
        return [];
    }
};

globalThis.launchApp = function(packageName) {
    if (typeof host_launch_app === "function" && packageName) {
        host_launch_app(String(packageName));
    }
};

globalThis.focusInput = function(id) {
    if (typeof host_focus_input === "function") {
        host_focus_input(String(id || ""));
    }
};

globalThis.blurInput = function() {
    if (typeof host_blur_input === "function") {
        host_blur_input();
    }
};

globalThis.getLocalTime = function() {
    try {
        return JSON.parse(host_get_local_time());
    } catch(e) {
        const d = new Date();
        return {
            time: d.toLocaleTimeString("tr-TR"),
            date: d.toLocaleDateString("tr-TR"),
            hours: d.getHours(),
            minutes: d.getMinutes(),
            seconds: d.getSeconds(),
            timestamp: d.getTime()
        };
    }
};

try {
    var defaultSettings = JSON.parse(host_get_default_settings());
    globalThis.defaultSettings = defaultSettings;
} catch(e) {
    var defaultSettings = {};
    globalThis.defaultSettings = defaultSettings;
}
