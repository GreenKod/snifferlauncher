// Default Interface - Main Entry Point

function Root() {
    const isLandscape = vw(100) > vh(100);

    if (isLandscape) {
        // Yatay Ekran (Landscape): Sol taraf %84 Grid, Sağ taraf %16 Dikey Görev Yöneticisi Şeridi
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

            // Dikey (Yukarıdan Aşağı) Görev Yöneticisi Alanı
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

    // Dikey Ekran (Portrait): Üst taraf %84 Grid, Alt taraf %16 Yatay Görev Yöneticisi Şeridi
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

globalThis.onEvent = function (eventJsonString) {
    const e = JSON.parse(eventJsonString);

    if (e.type === "WindowResized") {
        SnifferUI.forceUpdate();
        return "[]";
    }

    return "[]";
};

SnifferUI.start(Root, state);
