// Module List Component

function ModuleListComponent() {
    return [
        Container("list_header", {
            width: pct(90),
            height: "Auto",
            justify_content: "Start",
            align_items: "Start",
            padding: padXY(vmin(2.0), vmin(0.0)),
        }, [
            Label("list_title", "Recent Modules", {
                text_color: hex("#FFFFFF"),
                text_size: vmin(5.0),
                width: "Auto",
            }),
        ]),

        ...state.items.map((n, i) =>
            Container("list_item_" + i, {
                width: pct(90),
                height: px(vmin(15.0)),
                background_color: hex("#1E1E1E"),
                border_radius: vmin(3.0),
                flex_direction: "Row",
                align_items: "Center",
                justify_content: "Start",
                padding: padXY(vmin(0.0), vmin(4.0)),
                gap: vmin(4.0),
            }, [
                Container("item_icon_" + i, {
                    width: px(vmin(8.0)),
                    height: px(vmin(8.0)),
                    border_radius: vmin(4.0),
                    background_color: hex("#FF0055"),
                }, []),
                Label("item_text_" + i, "Module " + n, {
                    text_size: vmin(4.5),
                    text_color: hex("#DDDDDD"),
                    width: "Auto",
                })
            ])
        ),

        Container("bottom_pad", {
            width: pct(100),
            height: px(vmin(20.0)),
        }, [])
    ];
}
