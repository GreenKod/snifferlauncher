// =============================================================================
// System & Application Launcher Helpers
// =============================================================================

function launchApp(packageName) {
    if (typeof host_launch_app === "function" && packageName) {
        host_launch_app(String(packageName));
    }
}

function requestDefaultLauncher() {
    if (typeof host_request_default_launcher === "function") {
        host_request_default_launcher();
    }
}

function focusInput(id) {
    if (typeof host_focus_input === "function") {
        host_focus_input(String(id || ""));
    }
}

function blurInput() {
    if (typeof host_blur_input === "function") {
        host_blur_input();
    }
}

function getApplicationList() {
    if (typeof host_get_application_list === "function") {
        try {
            return JSON.parse(host_get_application_list());
        } catch (e) {
            return [];
        }
    }
    return [];
}

const Vault = globalThis.Vault || {
    get: function(k, d) {
        if (typeof host_vault_get !== "function") return d !== undefined ? d : null;
        try {
            const raw = host_vault_get(String(k));
            return raw ? JSON.parse(raw) : (d !== undefined ? d : null);
        } catch(e) { return d !== undefined ? d : null; }
    },
    set: function(k, v) {
        if (typeof host_vault_set !== "function") return false;
        try { return host_vault_set(String(k), JSON.stringify(v)); } catch(e) { return false; }
    },
    delete: function(k) {
        return typeof host_vault_delete === "function" ? host_vault_delete(String(k)) : false;
    },
    queryApps: function(p) {
        if (typeof host_vault_query_apps !== "function") return { apps: [], total_count: 0, page: 0, total_pages: 1 };
        try { return JSON.parse(host_vault_query_apps(JSON.stringify(p || {}))); } catch(e) { return { apps: [], total_count: 0, page: 0, total_pages: 1 }; }
    },
    keys: function(p) {
        if (typeof host_vault_keys !== "function") return [];
        try { return JSON.parse(host_vault_keys(String(p || ""))); } catch(e) { return []; }
    },
    subscribe: function(k, cb) {
        if (typeof subscribeChannel === "function") subscribeChannel("vault.changed:" + k, cb);
    },
    saveFile: function(fileName, base64Content) {
        return typeof host_vault_save_file === "function" ? host_vault_save_file(String(fileName), String(base64Content)) : "";
    },
    readFile: function(fileName) {
        return typeof host_vault_read_file === "function" ? host_vault_read_file(String(fileName)) : null;
    },
    deleteFile: function(fileName) {
        return typeof host_vault_delete_file === "function" ? host_vault_delete_file(String(fileName)) : false;
    },
    listFiles: function() {
        if (typeof host_vault_list_files !== "function") return [];
        try { return JSON.parse(host_vault_list_files()); } catch(e) { return []; }
    },
    getFileUrl: function(fileName) {
        return "vault://" + String(fileName).replace(/^vault:\/\//, "");
    }
};
