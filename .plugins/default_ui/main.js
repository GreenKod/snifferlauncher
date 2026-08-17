// Default Interface - Main Entry Point
// Rust scroll fizik motorunu kullanır.
// JS'in isDragging / dragOffset / kinetic scroll takip etmesine gerek yok.

function getDockElement(isLandscape) {
    let dockUI = null;
    if (typeof callApi === "function") {
        try {
            dockUI = callApi("dock.getUI", { isLandscape: isLandscape });
        } catch (e) {
            dockUI = null;
        }
    }
    if (dockUI) {
        return dockUI;
    }

    // Fallback if dock plugin is still initializing
    return Container("dock_fallback_area", {
        width: isLandscape ? px(vw(13.0)) : pct(100),
        height: isLandscape ? pct(100) : px(vh(18.0)),
        justify_content: "Center",
        align_items: "Center",
    }, [
        Label("dock_fallback_label", "...", {
            text_color: hex("#666666"),
            text_size: vmin(2.5),
            width: "Auto",
        })
    ]);
}

function Root() {
    const isLandscape = vw(100) > vh(100);

    if (isLandscape) {
        return Container("root", {
            width: pct(100),
            height: pct(100),
            position: "Relative",
            background_color: state.bgColor ? hex(state.bgColor) : undefined,
            flex_direction: "Row",
            justify_content: "Start",
            align_items: "Stretch",
        }, [
            AppGridComponent(),
            getDockElement(true)
        ]);
    }

    return Container("root", {
        width: pct(100),
        height: pct(100),
        position: "Relative",
        background_color: state.bgColor ? hex(state.bgColor) : undefined,
        flex_direction: "Column",
        justify_content: "Start",
        align_items: "Stretch",
    }, [
        AppGridComponent(),
        getDockElement(false)
    ]);
}

// Eklentiler Arası İletişim (IPC): Dock eklentisinden gelen yayınları dinle
subscribeChannel("dock.ready", function() {
    SnifferUI.forceUpdate();
    if (typeof broadcastEvent === "function") {
        broadcastEvent("default_ui.stats", { appCount: state.allApps.length });
    }
});

subscribeChannel("dock.stateChanged", function() {
    SnifferUI.forceUpdate();
});

subscribeChannel("dock.textChanged", function(data) {
    if (data && typeof data.text === "string") {
        state.searchQuery = data.text;
        SnifferUI.forceUpdate();
        if (typeof broadcastEvent === "function") {
            broadcastEvent("default_ui.stats", { appCount: state.filteredApps.length });
        }
    }
});

// Native Data Vault Reaktif Dinleyici (Polling yerine)
if (typeof Vault !== "undefined" && typeof Vault.subscribe === "function") {
    Vault.subscribe("system.apps", function() {
        state.refreshApps();
        SnifferUI.forceUpdate();
        if (typeof broadcastEvent === "function") {
            broadcastEvent("default_ui.stats", { appCount: state.allApps.length });
        }
    });
}

if (typeof broadcastEvent === "function") {
    broadcastEvent("default_ui.stats", { appCount: state.allApps.length });
}

let _hasInitialRefreshed = false;

// Rust tarafından snap tamamlandığında çağrılır.
// Sayfa indicator dots'unu güncellemek için kullanılır.
function onPageChanged(pageData) {
    const data = typeof pageData === "string" ? JSON.parse(pageData) : pageData;
    if (typeof data.page === "number") {
        state.currentPage = data.page;
        SnifferUI.forceUpdate();
    }
}

globalThis.onEvent = function (eventJsonString) {
    if (!_hasInitialRefreshed) {
        _hasInitialRefreshed = true;
        // Check if we still have mock apps before refreshing on first interaction
        if (state.allApps.length > 0 && state.allApps[0].id === "mock_app_1") {
            let realApps = getApplicationList();
            if (realApps.length > 0) {
                state.refreshApps();
                SnifferUI.forceUpdate();
            }
        }
    }

    const e = JSON.parse(eventJsonString);

    if (e.type === "WindowResized") {
        // state.refreshApps() kaldırıldı! 
        // WindowResized (uygulamayı arka plandan geri alma) durumlarında 
        // 150+ uygulamanın JSON'unu tekrar parse edip UI ağacını baştan kurmak devasa bir LAG'a sebep oluyordu.
        // Taffy layout motoru forceUpdate ile boyutları zaten dinamik hesaplar.
        SnifferUI.forceUpdate();
        return "[]";
    }

    // PageSnapped: Rust fizik motoru snap tamamlandığını bildirdi
    if (e.type === "PageSnapped") {
        if (typeof onPageChanged === "function") {
            onPageChanged({ page: e.page });
        }
        return "[]";
    }

    if (e.type === "Click") {
        const idStr = String(e.id || "");

        // Ana ızgara uygulaması tıklaması (Dock tıklamalarını Dock eklentisi onEvent ile kendisi karşılar)
        const pkg = state.getAppPackageByHash(idStr);
        if (typeof host_log === "function") {
            host_log("[SnifferLauncher JS] Click received for ID: " + idStr + " => Resolved Package: " + (pkg || "null"));
        }
        if (pkg) {
            launchApp(pkg);
        }
        return "[]";
    }

    // Scroll, PointerDown, PointerUp: app_grid_pager için Rust handles,
    // diğer scroll view'lar için normal akış devam eder.
    return "[]";
};

if (typeof subscribeChannel === "function") {
    subscribeChannel("dock.stateChanged", function() {
        SnifferUI.forceUpdate();
    });
    subscribeChannel("dock.ready", function() {
        SnifferUI.forceUpdate();
    });
}

SnifferUI.start(Root, state);
