// --- SnifferLauncher Inter-Plugin API ---

// Internal map: API name -> JS handler function
globalThis._apis = {};

/**
 * Register a named API endpoint for this plugin.
 * Other plugins can call it by name using callApi().
 *
 * @param {string} name - Unique dot-namespaced name, e.g. "store.get"
 * @param {function} handler - (payload: object) => any
 */
globalThis.registerApi = function(name, handler) {
    globalThis._apis[name] = handler;
    host_register_api(name);
};

/**
 * Call a named API exposed by any registered plugin (synchronous).
 *
 * @param {string} name - API name, e.g. "store.get"
 * @param {object} payload - JSON-serializable payload
 * @returns {object|null} - JSON-parsed response or null on failure/missing API
 */
globalThis.callApi = function(name, payload) {
    const json = host_call_api(name, JSON.stringify(payload ?? {}));
    if (json === null || json === undefined) return null;
    try { return JSON.parse(json); } catch(e) { return null; }
};

/**
 * Broadcast an event to ALL registered plugins (delivered next frame).
 *
 * @param {string} channel - Channel name, e.g. "theme.changed"
 * @param {object} data - JSON-serializable data
 */
globalThis.broadcastEvent = function(channel, data) {
    host_broadcast(channel, JSON.stringify(data ?? {}));
};

/**
 * Request runtime permissions for this plugin.
 *
 * @param {string[]} permissions
 * @returns {string[]} - Granted permission names.
 */
globalThis.requestPermissions = function(permissions) {
    return JSON.parse(host_request_permissions(permissions));
};

/**
 * Check whether a permission has already been granted.
 *
 * @param {string} permission
 * @returns {boolean}
 */
globalThis.hasPermission = function(permission) {
    return host_has_permission(permission);
};

// =============================================================================
// Native Data Vault API
// =============================================================================

globalThis.Vault = {
    /**
     * Get a value from the vault by key.
     * @param {string} key
     * @param {*} [defaultValue=null]
     * @returns {*}
     */
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

    /**
     * Store a value in the vault.
     * @param {string} key
     * @param {*} value
     * @returns {boolean}
     */
    set: function(key, value) {
        if (typeof host_vault_set !== "function") return false;
        try {
            const json = JSON.stringify(value);
            return host_vault_set(String(key), json);
        } catch(e) {
            return false;
        }
    },

    /**
     * Delete a key from the vault.
     * @param {string} key
     * @returns {boolean}
     */
    delete: function(key) {
        if (typeof host_vault_delete !== "function") return false;
        return host_vault_delete(String(key));
    },

    /**
     * Fast native search and pagination for installed applications in Rust memory.
     * @param {{ search?: string, page?: number, limit?: number }} [params]
     * @returns {{ apps: Array<{name:string, package_name:string}>, total_count: number, page: number, total_pages: number }}
     */
    queryApps: function(params) {
        if (typeof host_vault_query_apps !== "function") {
            return { apps: [], total_count: 0, page: 0, total_pages: 1 };
        }
        try {
            const json = host_vault_query_apps(JSON.stringify(params ?? {}));
            return JSON.parse(json);
        } catch(e) {
            return { apps: [], total_count: 0, page: 0, total_pages: 1 };
        }
    },

    /**
     * List all keys matching an optional prefix.
     * @param {string} [prefix=""]
     * @returns {string[]}
     */
    keys: function(prefix) {
        if (typeof host_vault_keys !== "function") return [];
        try {
            return JSON.parse(host_vault_keys(String(prefix || "")));
        } catch(e) {
            return [];
        }
    },

    /**
     * Subscribe to changes on a vault key.
     * @param {string} key
     * @param {function} callback
     */
    subscribe: function(key, callback) {
        if (typeof subscribeChannel === "function") {
            subscribeChannel("vault.changed:" + key, callback);
        }
    },

    /**
     * Save a binary file or image to the plugin's isolated file vault.
     * @param {string} fileName
     * @param {string} base64Content
     * @returns {string} - The "vault://fileName" URI for rendering.
     */
    saveFile: function(fileName, base64Content) {
        if (typeof host_vault_save_file !== "function") return "";
        return host_vault_save_file(String(fileName), String(base64Content));
    },

    /**
     * Read a binary file or image from the plugin's isolated file vault as Base64.
     * @param {string} fileName
     * @returns {string|null} - Base64 string or null if not found.
     */
    readFile: function(fileName) {
        if (typeof host_vault_read_file !== "function") return null;
        return host_vault_read_file(String(fileName));
    },

    /**
     * Delete a file from the plugin's isolated file vault.
     * @param {string} fileName
     * @returns {boolean}
     */
    deleteFile: function(fileName) {
        if (typeof host_vault_delete_file !== "function") return false;
        return host_vault_delete_file(String(fileName));
    },

    /**
     * List all files in the plugin's isolated file vault.
     * @returns {string[]}
     */
    listFiles: function() {
        if (typeof host_vault_list_files !== "function") return [];
        try {
            return JSON.parse(host_vault_list_files());
        } catch(e) {
            return [];
        }
    },

    /**
     * Get the rendering URI for a file in the vault.
     * @param {string} fileName
     * @returns {string}
     */
    getFileUrl: function(fileName) {
        return "vault://" + String(fileName).replace(/^vault:\/\//, "");
    },

    /**
     * Get the active operating system theme.
     * @returns {{is_dark: boolean, mode: string, accent_color: string, bg_color: string, text_color: string, card_bg: string}}
     */
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

    /**
     * Check if system is currently in Dark Mode.
     * @returns {boolean}
     */
    isDarkMode: function() {
        const t = this.getTheme();
        return t ? t.is_dark !== false : true;
    },

    /**
     * Listen for system theme changes.
     * @param {function} callback
     */
    onThemeChange: function(callback) {
        this.subscribe("system.theme", callback);
    }
};

/**
 * Fetch the installed application list.
 *
 * Requires the plugin to declare the appropriate permission in its manifest.
 * @returns {Array<{name:string, package_name:string}>}
 */
globalThis.getApplicationList = function() {
    try {
        return JSON.parse(host_get_application_list());
    } catch(e) {
        return [];
    }
};

/**
 * Launch an installed application by package name.
 *
 * @param {string} packageName
 */
globalThis.launchApp = function(packageName) {
    if (typeof host_launch_app === "function" && packageName) {
        host_launch_app(String(packageName));
    }
};

/**
 * Request soft keyboard focus for an input field by ID.
 *
 * @param {string} id
 */
globalThis.focusInput = function(id) {
    if (typeof host_focus_input === "function") {
        host_focus_input(String(id || ""));
    }
};

/**
 * Blur/hide the soft keyboard.
 */
globalThis.blurInput = function() {
    if (typeof host_blur_input === "function") {
        host_blur_input();
    }
};

/**
 * Fetch the exact OS local system time.
 *
 * @returns {{ time: string, date: string, hours: number, minutes: number, seconds: number, timestamp: number }}
 */
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

/**
 * Default settings loaded from manifest.json ("defaultSettings" key).
 */
try {
    var defaultSettings = JSON.parse(host_get_default_settings());
    globalThis.defaultSettings = defaultSettings;
} catch(e) {
    var defaultSettings = {};
    globalThis.defaultSettings = defaultSettings;
}

// Timers Polyfill (setInterval, setTimeout, clearInterval, clearTimeout)
globalThis._timers = {};
globalThis._timerId = 1;

globalThis.setInterval = function(callback, delayMs) {
    const id = globalThis._timerId++;
    globalThis._timers[id] = {
        callback: callback,
        delay: (typeof delayMs === 'number' && delayMs >= 0) ? delayMs : 1000,
        lastRun: Date.now(),
        once: false
    };
    return id;
};

globalThis.clearInterval = function(id) {
    delete globalThis._timers[id];
};

globalThis.setTimeout = function(callback, delayMs) {
    const id = globalThis._timerId++;
    globalThis._timers[id] = {
        callback: callback,
        delay: (typeof delayMs === 'number' && delayMs >= 0) ? delayMs : 0,
        lastRun: Date.now(),
        once: true
    };
    return id;
};

globalThis.clearTimeout = function(id) {
    delete globalThis._timers[id];
};

globalThis._onTimerTick = function() {
    const now = Date.now();
    for (const id in globalThis._timers) {
        const timer = globalThis._timers[id];
        if (now - timer.lastRun >= timer.delay) {
            timer.lastRun = now;
            try { timer.callback(); } catch(e) {}
            if (timer.once) {
                delete globalThis._timers[id];
            }
        }
    }
};

// Broadcast Channel Subscribers Map
globalThis._broadcastListeners = {};

/**
 * Subscribe to a specific broadcast channel.
 *
 * @param {string} channel
 * @param {function} callback - (data: object) => void
 */
globalThis.subscribeChannel = function(channel, callback) {
    if (typeof callback !== "function") return;
    if (!globalThis._broadcastListeners[channel]) {
        globalThis._broadcastListeners[channel] = [];
    }
    globalThis._broadcastListeners[channel].push(callback);
};

/**
 * Unsubscribe all listeners from a broadcast channel.
 *
 * @param {string} channel
 */
globalThis.unsubscribeChannel = function(channel) {
    delete globalThis._broadcastListeners[channel];
};

/**
 * Register a broadcast listener (supports both wildcard function or (channel, callback)).
 *
 * @param {string|function} channelOrCallback
 * @param {function} [callback]
 */
globalThis.onBroadcast = function(channelOrCallback, callback) {
    if (typeof channelOrCallback === "function") {
        globalThis.subscribeChannel("*", channelOrCallback);
    } else if (typeof channelOrCallback === "string" && typeof callback === "function") {
        globalThis.subscribeChannel(channelOrCallback, callback);
    }
};

// SharedView Handshake Protocol & Pending Invitations Storage
globalThis._sharedViewPending = {};

/**
 * Request another plugin to draw inside a SharedView slot or invite another plugin to draw inside yours.
 *
 * @param {string} targetPluginId - Target plugin ID
 * @param {string} slotName - SharedView slot identifier
 * @param {object} payload - Custom data/parameters
 * @param {number} timeoutMs - Response timeout in milliseconds (default 3000ms)
 * @returns {Promise<{accepted: boolean, status: string, reason?: string, uiTree?: object}>}
 */
globalThis.requestSharedView = function(targetPluginId, slotName, payload, timeoutMs) {
    const tMs = (typeof timeoutMs === 'number' && timeoutMs > 0) ? timeoutMs : 3000;
    return new Promise(function(resolve) {
        const invitationId = "inv_" + Math.random().toString(36).substring(2, 10);
        
        const timer = setTimeout(function() {
            if (globalThis._sharedViewPending[invitationId]) {
                delete globalThis._sharedViewPending[invitationId];
                resolve({
                    accepted: false,
                    status: "timed_out",
                    reason: "Response timeout exceeded (" + tMs + "ms)"
                });
            }
        }, tMs);

        globalThis._sharedViewPending[invitationId] = { resolve: resolve, timer: timer };

        const payloadStr = JSON.stringify(payload ?? {});
        broadcastEvent("shared_view_invite", {
            invitationId: invitationId,
            targetPluginId: targetPluginId,
            slotName: slotName,
            payload: payloadStr,
            timeoutMs: tMs
        });
    });
};

/**
 * Accept a SharedView invitation and return the UI tree to draw in the slot.
 */
globalThis.acceptSharedView = function(invitationId, uiTree) {
    broadcastEvent("shared_view_response", {
        invitationId: invitationId,
        accepted: true,
        status: "accepted",
        uiTree: uiTree ?? null
    });
};

/**
 * Reject a SharedView invitation with a reason.
 */
globalThis.rejectSharedView = function(invitationId, reason) {
    broadcastEvent("shared_view_response", {
        invitationId: invitationId,
        accepted: false,
        status: "rejected",
        reason: reason ?? "Invitation rejected"
    });
};

/**
 * Override this callback in your plugin to handle incoming SharedView requests.
 */
globalThis.onRequestSharedView = function(invitation) {
    globalThis.rejectSharedView(invitation.invitationId, "No handler registered");
};

// Internal — Rust calls this to deliver a broadcast to this plugin.
globalThis._dispatchBroadcast = function(channel, payload_json) {
    try {
        const data = JSON.parse(payload_json);
        
        if (channel === "shared_view_invite") {
            if (data.targetPluginId && typeof globalThis.onRequestSharedView === 'function') {
                globalThis.onRequestSharedView({
                    invitationId: data.invitationId,
                    slotName: data.slotName,
                    payload: JSON.parse(data.payload ?? "{}")
                });
            }
        } else if (channel === "shared_view_response") {
            const pending = globalThis._sharedViewPending[data.invitationId];
            if (pending) {
                clearTimeout(pending.timer);
                delete globalThis._sharedViewPending[data.invitationId];
                pending.resolve({
                    accepted: !!data.accepted,
                    status: data.status,
                    reason: data.reason,
                    uiTree: data.uiTree
                });
            }
        }

        // 1. Channel-specific subscribers
        if (globalThis._broadcastListeners[channel]) {
            globalThis._broadcastListeners[channel].forEach(function(cb) {
                try { cb(data); } catch(e) {}
            });
        }

        // 2. Wildcard subscribers
        if (globalThis._broadcastListeners["*"]) {
            globalThis._broadcastListeners["*"].forEach(function(cb) {
                try { cb(channel, data); } catch(e) {}
            });
        }
    } catch(e) {}
};

// Internal — Rust calls this when another plugin invokes one of our APIs.
globalThis._handleApiCall = function(name, payload_json) {
    if (!globalThis._apis[name]) return null;
    try {
        const payload = JSON.parse(payload_json);
        const result = globalThis._apis[name](payload);
        return JSON.stringify(result ?? null);
    } catch(e) {
        return null;
    }
};
