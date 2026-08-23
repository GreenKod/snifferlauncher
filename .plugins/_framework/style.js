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
