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

// Expose state to the plugin scope as a convenience shorthand.
// Plugins should use:   state.myProp   (read)
//                       SnifferUI.setState({ myProp: newVal })   (write)

// =============================================================================
// Color helpers
// =============================================================================

/**
 * Convert a CSS hex color string to a Rust u32 ARGB value.
 * Supports "#RGB", "#RRGGBB", "#AARRGGBB".
 * @param {string} hex
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
 * Build an Image element.
 * @param {string} id
 * @param {string} src  - Path relative to the plugin root, e.g. ".plugins/default_ui/photo.jpg"
 * @param {object} style
 */
function Image(id, src, style) {
    return { Image: { id, src, style: style ?? {} } };
}

/**
 * Build a TextInput element.
 * @param {string}  id
 * @param {string}  value
 * @param {object}  opts   - { focused, style }
 */
function TextInput(id, value, opts) {
    const { focused = false, style = {} } = opts ?? {};
    return { TextInput: { id, value: String(value), focused, style } };
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
