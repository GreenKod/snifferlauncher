// Module List Component - 48 Applications Grid

function ModuleListComponent() {
    const query = (state.searchQuery || "").toLowerCase().trim();
    const filteredApps = state.apps.filter(app => {
        if (!query) return true;
        return app.name.toLowerCase().includes(query) || (app.package_name && app.package_name.toLowerCase().includes(query));
    });

    const colors = [
        [hex("#FF416C"), hex("#FF4B2B")],
        [hex("#1D2B64"), hex("#F8CDDA")],
        [hex("#6441A5"), hex("#2a0845")],
        [hex("#00B4DB"), hex("#0083B0")],
        [hex("#11998e"), hex("#38ef7d")],
        [hex("#FF8008"), hex("#FFC837")],
        [hex("#8E2DE2"), hex("#4A00E0")],
        [hex("#F5576C"), hex("#F093FB")],
    ];

    return [
        // App Grid Section Header
        Container("app_grid_header", {
            width: pct(90),
            height: "Auto",
            flex_direction: "Row",
            justify_content: "SpaceBetween",
            align_items: "Center",
            padding: padXY(vmin(2.0), vmin(0.0)),
        }, [
            Label("app_grid_title", `Applications (${filteredApps.length} / 48)`, {
                text_color: hex("#FFFFFF"),
                text_size: vmin(5.0),
                width: "Auto",
            }),
            Label("app_grid_subtitle", "First 48 Apps", {
                text_color: hex("#FF0055"),
                text_size: vmin(3.5),
                width: "Auto",
            }),
        ]),

        // App Grid Container (Horizontal Flex-Wrap Grid)
        Container("app_grid_container", {
            width: pct(90),
            height: "Auto",
            flex_direction: "Row",
            flex_wrap: "Wrap",
            justify_content: "SpaceBetween",
            gap: vmin(3.0),
        }, filteredApps.map((app, index) => {
            const gradient = colors[index % colors.length];
            const initialLetter = app.name ? app.name.charAt(0).toUpperCase() : "A";

            return Container("app_card_" + index, {
                width: px(vw(20.0)),
                height: px(vw(24.0)),
                background_color: hex("#1E1E1E"),
                border_radius: vmin(4.0),
                flex_direction: "Column",
                align_items: "Center",
                justify_content: "Center",
                padding: pad(vmin(2.0)),
                gap: vmin(1.5),
                shadow_color: hex("#44000000"),
                shadow_offset_y: vmin(0.8),
                shadow_spread: vmin(0.5),
            }, [
                // App Icon Circle Badge
                Container("app_icon_" + index, {
                    width: px(vmin(10.0)),
                    height: px(vmin(10.0)),
                    border_radius: vmin(5.0),
                    background_gradient: gradient,
                    justify_content: "Center",
                    align_items: "Center",
                }, [
                    Label("app_letter_" + index, initialLetter, {
                        text_color: hex("#FFFFFF"),
                        text_size: vmin(5.5),
                        width: "Auto",
                    })
                ]),
                // App Name Label
                Label("app_name_" + index, app.name, {
                    text_color: hex("#DDDDDD"),
                    text_size: vmin(3.2),
                    width: "Auto",
                })
            ]);
        })),

        // Bottom Spacer for FAB
        Container("bottom_pad", {
            width: pct(100),
            height: px(vmin(20.0)),
        }, [])
    ];
}
