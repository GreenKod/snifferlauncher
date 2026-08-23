// =============================================================================
// Native Data Vault API
// =============================================================================

globalThis.Vault = {
    get: function(key, defaultValue) {
        if (typeof host_vault_get !== "function") return defaultValue !== undefined ? defaultValue : null;
        try {
            const raw = host_vault_get(String(key));
            if (raw === null || raw === undefined) return defaultValue !== undefined ? defaultValue : null;
            return JSON.parse(raw);
        } catch(e) {
            return defaultValue !== undefined ? defaultValue : null;
        }
    },

    set: function(key, value) {
        if (typeof host_vault_set !== "function") return false;
        try {
            const json = JSON.stringify(value);
            return host_vault_set(String(key), json);
        } catch(e) {
            return false;
        }
    },

    delete: function(key) {
        if (typeof host_vault_delete !== "function") return false;
        return host_vault_delete(String(key));
    },

    queryApps: function(params) {
        if (typeof host_vault_query_apps !== "function") {
            return { apps: [], total_count: 0, page: 0, total_pages: 1 };
        }
        try {
            const json = host_vault_query_apps(JSON.stringify(params !== undefined && params !== null ? params : {}));
            return JSON.parse(json);
        } catch(e) {
            return { apps: [], total_count: 0, page: 0, total_pages: 1 };
        }
    },

    keys: function(prefix) {
        if (typeof host_vault_keys !== "function") return [];
        try {
            return JSON.parse(host_vault_keys(String(prefix || "")));
        } catch(e) {
            return [];
        }
    },

    subscribe: function(key, callback) {
        if (typeof subscribeChannel === "function") {
            subscribeChannel("vault.changed:" + key, callback);
        }
    },

    saveFile: function(fileName, base64Content) {
        if (typeof host_vault_save_file !== "function") return "";
        return host_vault_save_file(String(fileName), String(base64Content));
    },

    readFile: function(fileName) {
        if (typeof host_vault_read_file !== "function") return null;
        return host_vault_read_file(String(fileName));
    },

    deleteFile: function(fileName) {
        if (typeof host_vault_delete_file !== "function") return false;
        return host_vault_delete_file(String(fileName));
    },

    listFiles: function() {
        if (typeof host_vault_list_files !== "function") return [];
        try {
            return JSON.parse(host_vault_list_files());
        } catch(e) {
            return [];
        }
    },

    getFileUrl: function(fileName) {
        return "vault://" + String(fileName).replace(/^vault:\/\//, "");
    },

    getTheme: function() {
        return this.get("system.theme", {
            is_dark: true,
            mode: "dark",
            accent_color: "#38BDF8",
            bg_color: "#0F172A",
            text_color: "#F8FAFC",
            card_bg: "#1E293B"
        });
    },

    isDarkMode: function() {
        const t = this.getTheme();
        return t ? t.is_dark !== false : true;
    },

    onThemeChange: function(callback) {
        this.subscribe("system.theme", callback);
    }
};
