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
 * @param {string} id
 * @param {object} opts  - { scroll_x, scroll_y, scroll_sensitivity, dynamic_sensitivity,
 *                           momentum_scrolling, capture_drag, style }
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
// Default UI - State (4x7 Layout)

function loadInitialApps() {
    let installed = [];
    try {
        installed = typeof getApplicationList === "function" ? getApplicationList() : [];
    } catch (e) {
        installed = [];
    }

    if (Array.isArray(installed) && installed.length > 0) {
        return installed;
    }

    return [
        { id: "app_1", name: "Tarayıcı", package_name: "com.sniffer.browser" },
        { id: "app_2", name: "Ayarlar", package_name: "com.sniffer.settings" },
        { id: "app_3", name: "Galeri", package_name: "com.sniffer.gallery" },
        { id: "app_4", name: "Kamera", package_name: "com.sniffer.camera" },
        { id: "app_5", name: "Müzik", package_name: "com.sniffer.music" },
        { id: "app_6", name: "Dosyalar", package_name: "com.sniffer.files" },
        { id: "app_7", name: "Terminal", package_name: "com.sniffer.terminal" },
        { id: "app_8", name: "Hesap Makinesi", package_name: "com.sniffer.calc" },
        { id: "app_9", name: "Takvim", package_name: "com.sniffer.calendar" },
        { id: "app_10", name: "Saat", package_name: "com.sniffer.clock" },
        { id: "app_11", name: "Mesajlar", package_name: "com.sniffer.messages" },
        { id: "app_12", name: "Telefon", package_name: "com.sniffer.phone" },
        { id: "app_13", name: "Notlar", package_name: "com.sniffer.notes" },
        { id: "app_14", name: "Haritalar", package_name: "com.sniffer.maps" },
        { id: "app_15", name: "Mağaza", package_name: "com.sniffer.store" },
        { id: "app_16", name: "Hava Durumu", package_name: "com.sniffer.weather" }
    ];
}

const APPS_PER_PAGE = 28;

let state = {
    bgColor: "#0F0F12",
    searchQuery: "",
    isSearchFocused: false,
    scrollX: 0.0,
    scrollY: 0.0,

    currentPage: 1,
    appsPerPage: APPS_PER_PAGE,

    allApps: loadInitialApps(),

    get filteredApps() {
        const query = (this.searchQuery || "").toLowerCase().trim();
        return this.allApps.filter(app => {
            if (!query) return true;
            return (app.name && app.name.toLowerCase().includes(query))
                || (app.package_name && app.package_name.toLowerCase().includes(query));
        });
    },

    get apps() {
        const list = this.filteredApps;
        const startIndex = (this.currentPage - 1) * this.appsPerPage;
        return list.slice(startIndex, startIndex + this.appsPerPage);
    },

    get totalPages() {
        return Math.max(1, Math.ceil(this.filteredApps.length / this.appsPerPage));
    },

    setPage(p) {
        const maxPage = this.totalPages;
        if (p >= 1 && p <= maxPage) {
            this.currentPage = p;
        }
    }
};
// ============================================================
// AppLayout - 5x7 Dinamik Layout Sistemi
//
// Boşluk Kuralı:
//   gapUnit = vmin(20) / 6
//   X ekseni: appW = (vw(100) - 6*gap) / 5
//             (sağ+sol dış padding: 1*gap her biri, 4 iç gap)
//   Y ekseni: appH = (vh(70) - 8*gap) / 7
//             (üst+alt dış padding: 1*gap her biri, 6 iç gap)
//             → Alt %30 görev yöneticisi için ayrıldı
// ============================================================

const GRID_COLS = 5;
const GRID_ROWS = 7;
const TOTAL_APPS = GRID_COLS * GRID_ROWS;  // 35

// Temel boşluk birimi: en küçük eksenin %20'sini 6'ya böl
function computeLayout() {
    const gap = vmin(20.0 / 6.0);

    // Uygulama genişliği:
    // | gap | app | gap | app | gap | app | gap | app | gap | app | gap |
    // = 6 gap + 5 app = vw(100)
    const appW = (vw(100) - 6 * gap) / 5;

    // Uygulama yüksekliği (sadece üst %70 kullanılır):
    // | gap | app | gap | app | gap | ... | app | gap |
    // = 8 gap + 7 app = vh(70)
    const appH = (vh(70) - 8 * gap) / 7;

    const iconSize = Math.min(appW, appH) * 0.40;

    return { gap, appW, appH, iconSize };
}

const GRID_COLORS = [
    [hex("#FF416C"), hex("#FF4B2B")],
    [hex("#1D2B64"), hex("#F8CDDA")],
    [hex("#6441A5"), hex("#2a0845")],
    [hex("#00B4DB"), hex("#0083B0")],
    [hex("#11998e"), hex("#38ef7d")],
    [hex("#FF8008"), hex("#FFC837")],
    [hex("#8E2DE2"), hex("#4A00E0")],
    [hex("#F5576C"), hex("#F093FB")],
    [hex("#00c6ff"), hex("#0072ff")],
    [hex("#f953c6"), hex("#b91d73")],
    [hex("#43e97b"), hex("#38f9d7")],
    [hex("#fa709a"), hex("#fee140")],
];
ürüyoruz.
    return targetSize;
}
// App Grid — 5x7 Manuel Satır Bazlı Grid
// flex_wrap kullanmaz, her satır ayrı Container olarak oluşturulur

function AppCardComponent(app, index, layout) {
    const gradient = GRID_COLORS[index % GRID_COLORS.length];
    const letter = app.name ? app.name.charAt(0).toUpperCase() : "?";
    const { appW, appH, iconSize } = layout;

    const iconRadius = iconSize / 2;
    const iconFontSize = iconSize * 0.46;
    const nameFontSize = Math.min(appW, appH) * 0.30;
    const cardRadius = Math.min(appW, appH) * 0.12;

    return Container("card_" + index, {
        width: px(appW),
        height: px(appH),
        background_color: hex("#15151F"),
        border_radius: cardRadius,
        flex_direction: "Column",
        align_items: "Center",
        justify_content: "Center",
        gap: appH * 0.06,
        shadow_color: hex("#33000000"),
        shadow_offset_y: appH * 0.02,
        shadow_spread: appH * 0.03,
    }, [
        Container("icon_" + index, {
            width: px(iconSize),
            height: px(iconSize),
            border_radius: iconRadius,
            background_gradient: gradient,
            justify_content: "Center",
            align_items: "Center",
        }, [
            Label("letter_" + index, letter, {
                text_color: hex("#FFFFFF"),
                text_size: iconFontSize,
                width: "Auto",
            })
        ]),
        Label("name_" + index, app.name, {
            text_color: hex("#CCCCDD"),
            text_size: nameFontSize,
            width: "Auto",
        }),
    ]);
}

function AppGridComponent() {
    const layout = computeLayout();
    const { gap, appW, appH } = layout;

    const apps = state.apps.slice(0, TOTAL_APPS);

    // Her satırı ayrı Container olarak oluştur
    const rows = [];
    for (let r = 0; r < GRID_ROWS; r++) {
        const rowApps = apps.slice(r * GRID_COLS, (r + 1) * GRID_COLS);
        const rowCards = rowApps.map((app, c) =>
            AppCardComponent(app, r * GRID_COLS + c, layout)
        );

        rows.push(
            Container("row_" + r, {
                width: "Auto",
                height: px(appH),
                flex_direction: "Row",
                align_items: "Center",
                gap: gap,
            }, rowCards)
        );
    }

    return rows;
}
// Default Interface - Main Entry Point

function Root() {
    const layout = computeLayout();
    return Container("root", {
        width: pct(100),
        height: pct(100),
        background_color: hex(state.bgColor),
        flex_direction: "Row",
        flex_wrap: "Wrap",
        justify_content: "Start",
        padding: pad(layout.gap), // Dış boşluk da GAP kadar (üst, alt, sağ, sol)
    }, [
        ...AppGridComponent(),
    ]);
}

subscribeChannel("clock.secondChanged", function(eventData) {});

globalThis.onEvent = function (eventJsonString) {
    const e = JSON.parse(eventJsonString);

    if (e.WindowResized || "WindowResized" in e) {
        SnifferUI.forceUpdate();
        return "[]";
    }

    return "[]";
};

SnifferUI.start(Root, state);
