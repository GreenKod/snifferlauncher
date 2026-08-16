// =============================================================================
// SnifferUI — React-like mini framework for SnifferLauncher plugins
// =============================================================================
// Usage in your plugin's main.js:
//   1. Declare your state:    let state = { count: 0, text: "Hi" };
//   2. Write a render fn:     function App() { return Container("root", {}, [...]) }
//   3. Start the app:         SnifferUI.start(App, state);
//   4. Handle events:         globalThis.onEvent = e => { SnifferUI.setState({ count: state.count + 1 }); }
//
// The framework automatically calls host_set_ui() whenever setState() is used.
// =============================================================================

const SnifferUI = (function () {
    let _renderFn = null;
    let _state = {};

    /** Internal: renders and pushes the UI to Rust. */
    function _commit() {
        if (_renderFn) {
            host_set_ui(JSON.stringify(_renderFn()));
        }
    }

    return {
        /**
         * Start the app. Renders immediately.
         * @param {function} renderFn - The root render function (e.g. `App`)
         * @param {object}   initialState - Your initial state object
         */
        start(renderFn, initialState) {
            _renderFn = renderFn;
            _state    = initialState ?? {};
            _commit();
        },

        /**
         * Merge a partial state patch and re-render.
         * Works just like React's setState (shallow merge).
         * @param {object} patch
         */
        setState(patch) {
            Object.assign(_state, patch);
            _commit();
        },

        /** Force a re-render without changing state. */
        forceUpdate() {
            _commit();
        },
    };
})();

// =============================================================================
// Channel Subscription Helper System
// =============================================================================

const _channelSubscribers = {};

/**
 * Subscribe to a specific broadcast channel name.
 *
 * @param {string}   channelName - Name of the channel to subscribe to
 * @param {function} callback    - Callback function called with payload data
 */
function subscribeChannel(channelName, callback) {
    if (!_channelSubscribers[channelName]) {
        _channelSubscribers[channelName] = [];
    }
    _channelSubscribers[channelName].push(callback);
}

/**
 * Unsubscribe from a broadcast channel name.
 *
 * @param {string} channelName
 */
function unsubscribeChannel(channelName) {
    delete _channelSubscribers[channelName];
}

// Automatic global broadcast router
globalThis.onBroadcast = function(channel, data) {
    if (_channelSubscribers[channel]) {
        _channelSubscribers[channel].forEach(function(cb) {
            try { cb(data); } catch(e) {}
        });
    }
};

// =============================================================================
// Color helpers
// =============================================================================

/**
 * Convert a CSS hex color string to a Rust u32 ARGB value.
 * Supports "#RGB", "#RRGGBB", "#AARRGGBB".
 * @param {string} hexStr
 * @returns {number}
 */
function hex(hexStr) {
    let h = hexStr.replace(/^#/, "");
    if (h.length === 3) h = h.split("").map(c => c + c).join("");
    if (h.length === 6) h = "FF" + h;
    return parseInt(h, 16) >>> 0;
}

// Backwards-compat alias
function hexToColor(h) { return hex(h); }

// =============================================================================
// Functional UI builders — the "JSX" of SnifferUI
// =============================================================================

/**
 * Build a Container element.
 * @param {string}   id
 * @param {object}   style
 * @param {Array}    children
 */
function Container(id, style, children) {
    return { Container: { id, style: style ?? {}, children: children ?? [] } };
}

/**
 * Build a Label element.
 * @param {string} id
 * @param {string} text
 * @param {object} style
 */
function Label(id, text, style) {
    return { Label: { id, text: String(text), style: style ?? {} } };
}

/**
 * Build a TextInput element.
 * @param {string}  id
 * @param {string}  value
 * @param {object}  opts  - { focused, placeholder, style }
 */
function TextInput(id, value, opts) {
    const { focused = false, placeholder = "", style = {} } = opts ?? {};
    return {
        TextInput: {
            id,
            value: String(value),
            placeholder: String(placeholder),
            focused: !!focused,
            style,
        },
    };
}

/**
 * Build an Image element.
 * @param {string} id
 * @param {string} src   - Relative path, absolute path, or "app-icon://<pkg>"
 * @param {object} style
 */
function Image(id, src, style) {
    return { Image: { id, src: String(src), style: style ?? {} } };
}

/**
 * Build a ScrollView element.
 *
 * Temel parametreler:
 * @param {string} id
 * @param {object} opts  - {
 *   scroll_x, scroll_y,
 *   scroll_sensitivity, dynamic_sensitivity,
 *   momentum_scrolling, capture_drag,
 *   style,
 *   --- Rust-managed scroll physics ---
 *   snap_x: number|null,      // Yatay snap aralığı px (pager modu). null = serbest
 *   snap_y: number|null,      // Dikey snap aralığı px. null = serbest
 *   rubber_band: number|null, // Kenar elastikiyeti 0.0–1.0. null = sert kenar
 *   page_count: number|null,  // Toplam sayfa sayısı (snap_x ile)
 *   on_snap: string|null,     // Snap tamamlanınca JS callback adı
 * }
 * @param {Array}  children
 */
function ScrollView(id, opts, children) {
    const {
        scroll_x = 0.0,
        scroll_y = 0.0,
        scroll_sensitivity = 1.0,
        dynamic_sensitivity = true,
        momentum_scrolling = true,
        capture_drag = true,
        style = {},
        // Rust-managed physics params
        snap_x = null,
        snap_y = null,
        rubber_band = null,
        page_count = null,
        on_snap = null,
    } = opts ?? {};
    return {
        ScrollView: {
            id,
            scroll_x,
            scroll_y,
            scroll_sensitivity,
            dynamic_sensitivity,
            momentum_scrolling,
            capture_drag,
            style,
            children: children ?? [],
            snap_x,
            snap_y,
            rubber_band,
            page_count,
            on_snap,
        },
    };
}

/**
 * Build a Checkbox element.
 * @param {string}  id
 * @param {boolean} checked
 * @param {object}  style
 */
function Checkbox(id, checked, style) {
    return { Checkbox: { id, checked: !!checked, style: style ?? {} } };
}

/**
 * Build a Slider element.
 * @param {string} id
 * @param {number} value
 * @param {number} min
 * @param {number} max
 * @param {object} style
 */
function Slider(id, value, min, max, style) {
    return { Slider: { id, value, min, max, style: style ?? {} } };
}

/**
 * Build a ProgressBar element.
 * @param {string} id
 * @param {number} value
 * @param {number} max
 * @param {object} style
 */
function ProgressBar(id, value, max, style) {
    return { ProgressBar: { id, value, max, style: style ?? {} } };
}

/**
 * Build a SharedView element for cross-plugin shared layout drawing.
 * @param {string}      id
 * @param {string|null} targetPluginId - Target plugin ID
 * @param {string|null} slotName       - SharedView slot identifier
 * @param {object}      style
 * @param {Array}       children       - Fallback elements if pending/rejected
 */
function SharedView(id, targetPluginId, slotName, style, children) {
    return {
        SharedView: {
            id,
            target_plugin: targetPluginId ? String(targetPluginId) : null,
            slot_name: slotName ? String(slotName) : null,
            style: style ?? {},
            children: children ?? []
        }
    };
}

// =============================================================================
// Layout shorthand helpers
// =============================================================================

/** Pixel dimension */
function px(n)  { return { Pixels: n }; }

/** Percent dimension */
function pct(n) { return { Percent: n }; }

/** Padding/margin shorthand — all sides equal */
function pad(all) {
    return { top: all, bottom: all, left: all, right: all };
}

/** Padding/margin shorthand — vertical / horizontal */
function padXY(v, h) {
    return { top: v, bottom: v, left: h, right: h };
}

// =============================================================================
// Responsive (Viewport) sizing helpers
// =============================================================================

/** % of screen width (e.g. vw(5) = 5% of width) */
function vw(percent) {
    return (host_screen_width() * percent) / 100.0;
}

/** % of screen height (e.g. vh(10) = 10% of height) */
function vh(percent) {
    return (host_screen_height() * percent) / 100.0;
}

/** The smaller of vw or vh */
function vmin(percent) {
    return Math.min(vw(percent), vh(percent));
}

/** The larger of vw or vh */
function vmax(percent) {
    return Math.max(vw(percent), vh(percent));
}

/** Launches an installed Android application by package name */
function launchApp(packageName) {
    if (typeof host_launch_app === "function" && packageName) {
        host_launch_app(String(packageName));
    }
}

/** Requests default home/launcher application role from Android system */
function requestDefaultLauncher() {
    if (typeof host_request_default_launcher === "function") {
        host_request_default_launcher();
    }
}

/** Requests soft keyboard focus for an input field */
function focusInput(id) {
    if (typeof host_focus_input === "function") {
        host_focus_input(String(id || ""));
    }
}

/** Blurs/hides the soft keyboard */
function blurInput() {
    if (typeof host_blur_input === "function") {
        host_blur_input();
    }
}

/** Retrieves the list of installed Android applications from the host system */
function getApplicationList() {
    if (typeof host_get_application_list === "function") {
        try {
            const jsonStr = host_get_application_list();
            return JSON.parse(jsonStr);
        } catch (e) {
            return [];
        }
    }
    return [];
}

/** Native Data Vault for high-performance memory storage & search */
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
    }
};

// =============================================================================
// Master Design System & Theme Tokens
// =============================================================================

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
    radius:  { sm: 8, md: 12, lg: 16 }
};

/**
 * Standardized Card Widget Component Template for all plugins.
 * Guarantees unified border radius, background color, padding, and layout grid.
 *
 * @param {object} opts - { id, title, subtitle, content, style }
 */
function CardWidget(opts) {
    const { id = "card-widget", title = "", subtitle = "", content = [], style = {} } = opts ?? {};
    
    const children = [];
    if (title) {
        children.push(Label(`${id}-title`, title, {
            text_color: Theme.colors.accent,
            text_size: vmin(4.0),
            width: "Auto"
        }));
    }
    if (subtitle) {
        children.push(Label(`${id}-sub`, subtitle, {
            text_color: Theme.colors.textSub,
            text_size: vmin(3.0),
            width: "Auto"
        }));
    }
    if (Array.isArray(content)) {
        children.push(...content);
    } else if (content) {
        children.push(content);
    }

    return Container(id, {
        background_color: Theme.colors.bgCard,
        border_radius: vmin(4.0),
        border_width: vmin(0.2),
        border_color: Theme.colors.border,
        padding: pad(vmin(3.5)),
        flex_direction: "Column",
        gap: vmin(2.0),
        overflow_hidden: true,
        ...style
    }, children);
}
