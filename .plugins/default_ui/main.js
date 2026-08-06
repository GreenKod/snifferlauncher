// Default Interface - Main Entry Point
// Rust scroll fizik motorunu kullanır.
// JS'in isDragging / dragOffset / kinetic scroll takip etmesine gerek yok.

function Root() {
    const isLandscape = vw(100) > vh(100);

    if (isLandscape) {
        return Container("root", {
            width: pct(100),
            height: pct(100),
            position: "Relative",
            background_color: hex(state.bgColor),
            flex_direction: "Row",
            justify_content: "Start",
            align_items: "Stretch",
        }, [
            ...AppGridComponent(),

            Container("task_manager_reserved_area", {
                width: px(vw(16.0)),
                height: pct(100),
                background_color: hex("#0B0B0E"),
                flex_direction: "Column",
                justify_content: "Center",
                align_items: "Center",
                gap: vh(1.2),
            }, [
                Label("tm_reserved_l1", "G Ö R E V", {
                    text_color: hex("#333344"),
                    text_size: vw(1.4),
                    width: "Auto",
                }),
                Label("tm_reserved_l2", "Y Ö N E T İ C İ S İ", {
                    text_color: hex("#333344"),
                    text_size: vw(1.2),
                    width: "Auto",
                }),
                Label("tm_reserved_l3", "A L A N I", {
                    text_color: hex("#333344"),
                    text_size: vw(1.4),
                    width: "Auto",
                }),
                Label("tm_reserved_l4", "(%16)", {
                    text_color: hex("#333344"),
                    text_size: vw(1.2),
                    width: "Auto",
                })
            ])
        ]);
    }

    return Container("root", {
        width: pct(100),
        height: pct(100),
        position: "Relative",
        background_color: hex(state.bgColor),
        flex_direction: "Column",
        justify_content: "Start",
        align_items: "Stretch",
    }, [
        ...AppGridComponent(),

        Container("task_manager_reserved_area", {
            width: pct(100),
            height: px(vh(16.0)),
            background_color: hex("#0B0B0E"),
            justify_content: "Center",
            align_items: "Center",
        }, [
            Label("tm_reserved_label", "Görev Yöneticisi Alanı (%16)", {
                text_color: hex("#333344"),
                text_size: vmin(4.0),
                width: "Auto",
            })
        ])
    ]);
}

subscribeChannel("clock.secondChanged", function(eventData) {});

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
        state.refreshApps();
    }

    const e = JSON.parse(eventJsonString);

    if (e.type === "WindowResized") {
        state.refreshApps();
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

    if (e.type === "Click" && e.id) {
        const pkg = state.getAppPackageByHash(String(e.id));
        if (pkg) {
            launchApp(pkg);
        }
        return "[]";
    }

    // Scroll, PointerDown, PointerUp: app_grid_pager için Rust handles,
    // diğer scroll view'lar için normal akış devam eder.
    return "[]";
};

SnifferUI.start(Root, state);
