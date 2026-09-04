// =============================================================================
// SnifferUI — React-like mini framework for SnifferLauncher plugins
// =============================================================================

const SnifferUI = (function () {
    let _renderFn = null;
    let _state = {};

    function _commit() {
        if (_renderFn) {
            host_set_ui(JSON.stringify(_renderFn()));
        }
    }

    return {
        start(renderFn, initialState) {
            _renderFn = renderFn;
            _state    = initialState !== undefined && initialState !== null ? initialState : {};
            _commit();
        },

        setState(patch) {
            Object.assign(_state, patch);
            _commit();
        },

        forceUpdate() {
            _commit();
        },
    };
})();
// =============================================================================
// Channel Subscription Helper System
// =============================================================================

const _channelSubscribers = {};

function subscribeChannel(channelName, callback) {
    if (!_channelSubscribers[channelName]) {
        _channelSubscribers[channelName] = [];
    }
    _channelSubscribers[channelName].push(callback);
}

function unsubscribeChannel(channelName) {
    delete _channelSubscribers[channelName];
}

globalThis.onBroadcast = function(channel, data) {
    if (_channelSubscribers[channel]) {
        _channelSubscribers[channel].forEach(function(cb) {
            try { cb(data); } catch(e) {}
        });
    }
};
// =============================================================================
// Color & Layout Style Dimension Helpers
// =============================================================================

function hex(hexStr) {
    let h = hexStr.replace(/^#/, "");
    if (h.length === 3) h = h.split("").map(c => c + c).join("");
    if (h.length === 6) h = "FF" + h;
    return parseInt(h, 16) >>> 0;
}

function hexToColor(h) { return hex(h); }

function px(n)  { return { Pixels: n }; }
function pct(n) { return { Percent: n }; }

function pad(all) {
    return { top: all, bottom: all, left: all, right: all };
}

function padXY(v, h) {
    return { top: v, bottom: v, left: h, right: h };
}

function vw(percent) {
    return (host_screen_width() * percent) / 100.0;
}

function vh(percent) {
    return (host_screen_height() * percent) / 100.0;
}

function vmin(percent) {
    return Math.min(vw(percent), vh(percent));
}

function vmax(percent) {
    return Math.max(vw(percent), vh(percent));
}

const Theme = {
    colors: {
        bgCard:     hex("#1E293B"),
        bgCardAlt:  hex("#0F172A"),
        textMain:   hex("#F8FAFC"),
        textSub:    hex("#94A3B8"),
        accent:     hex("#38BDF8"),
        success:    hex("#10B981"),
        border:     hex("#334155")
    },
    spacing: { xs: 4, sm: 8, md: 16, lg: 24 },
    radius:  { sm: 8, md: 12, lg: 16 },

    getSystemTheme: function() {
        return (typeof Vault !== "undefined" ? Vault.get("system.theme", null) : null) || {
            is_dark: true,
            mode: "dark",
            accent_color: "#38BDF8",
            bg_color: "#0F172A",
            text_color: "#F8FAFC",
            card_bg: "#1E293B"
        };
    },

    isDarkMode: function() {
        const t = this.getSystemTheme();
        return t ? t.is_dark !== false : true;
    },

    onSystemThemeChange: function(callback) {
        if (typeof Vault !== "undefined" && typeof Vault.subscribe === "function") {
            Vault.subscribe("system.theme", callback);
        }
    }
};
// =============================================================================
// Functional UI builders — the "JSX" of SnifferUI
// =============================================================================

function Container(id, style, children) {
    return { Container: { id: id, style: style || {}, children: children || [] } };
}

function Label(id, text, style) {
    return { Label: { id: id, text: String(text), style: style || {} } };
}

function TextInput(id, value, opts) {
    opts = opts || {};
    return {
        TextInput: {
            id: id,
            value: String(value),
            placeholder: String(opts.placeholder || ""),
            focused: !!opts.focused,
            style: opts.style || {},
        },
    };
}

function Image(id, src, style) {
    return { Image: { id: id, src: String(src), style: style || {} } };
}

function ScrollView(id, opts, children) {
    opts = opts || {};
    return {
        ScrollView: {
            id: id,
            scroll_x: opts.scroll_x !== undefined ? opts.scroll_x : 0.0,
            scroll_y: opts.scroll_y !== undefined ? opts.scroll_y : 0.0,
            scroll_sensitivity: opts.scroll_sensitivity !== undefined ? opts.scroll_sensitivity : 1.0,
            dynamic_sensitivity: opts.dynamic_sensitivity !== undefined ? opts.dynamic_sensitivity : true,
            momentum_scrolling: opts.momentum_scrolling !== undefined ? opts.momentum_scrolling : true,
            capture_drag: opts.capture_drag !== undefined ? opts.capture_drag : true,
            style: opts.style || {},
            children: children || [],
            snap_x: opts.snap_x !== undefined ? opts.snap_x : null,
            snap_y: opts.snap_y !== undefined ? opts.snap_y : null,
            rubber_band: opts.rubber_band !== undefined ? opts.rubber_band : null,
            page_count: opts.page_count !== undefined ? opts.page_count : null,
            on_snap: opts.on_snap !== undefined ? opts.on_snap : null,
        },
    };
}

function Checkbox(id, checked, style) {
    return { Checkbox: { id: id, checked: !!checked, style: style || {} } };
}

function Slider(id, value, min, max, style) {
    return { Slider: { id: id, value: value, min: min, max: max, style: style || {} } };
}

function ProgressBar(id, value, max, style) {
    return { ProgressBar: { id: id, value: value, max: max, style: style || {} } };
}

function SharedView(id, targetPluginId, slotName, style, children) {
    return {
        SharedView: {
            id: id,
            target_plugin: targetPluginId ? String(targetPluginId) : null,
            slot_name: slotName ? String(slotName) : null,
            style: style || {},
            children: children || []
        }
    };
}

function CardWidget(opts) {
    opts = opts || {};
    const id = opts.id || "card-widget";
    const title = opts.title || "";
    const subtitle = opts.subtitle || "";
    const content = opts.content || [];
    const customStyle = opts.style || {};

    const children = [];
    if (title) {
        children.push(Label(id + "-title", title, {
            text_color: Theme.colors.accent,
            text_size: vmin(4.0),
            width: "Auto"
        }));
    }
    if (subtitle) {
        children.push(Label(id + "-sub", subtitle, {
            text_color: Theme.colors.textSub,
            text_size: vmin(3.0),
            width: "Auto"
        }));
    }
    if (Array.isArray(content)) {
        for (let i = 0; i < content.length; i++) {
            children.push(content[i]);
        }
    } else if (content) {
        children.push(content);
    }

    const mergedStyle = Object.assign({
        background_color: Theme.colors.bgCard,
        border_radius: vmin(4.0),
        border_width: vmin(0.2),
        border_color: Theme.colors.border,
        padding: pad(vmin(3.5)),
        flex_direction: "Column",
        gap: vmin(2.0),
        overflow_hidden: true
    }, customStyle);

    return Container(id, mergedStyle, children);
}
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
