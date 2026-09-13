// Dock Bar Virtual DOM Generator
function getDockContainer(isLandscape) {
    if (isLandscape) {
        // Landscape Mode: Right Vertical Ergonomic Dock (vmin scaled)
        return Container("dock_root_container", {
            width: px(vmin(14.0)),
            height: pct(100),
            background_color: hex("#B80F172A"),
            border_radius: vmin(4.0),
            border_width: px(1.0),
            border_color: hex("#33FFFFFF"),
            flex_direction: "Column",
            justify_content: "Center",
            align_items: "Center",
            padding: padXY(vmin(1.0), vmin(1.0)),
            gap: vmin(2.5),
        }, dockState.essentialApps.map(function(app) {
            const iconSize = vmin(9.5);
            const iconRadius = iconSize / 2.0;
            const touchW = vmin(12.5);
            const touchH = vmin(12.5);
            
            return Container("dock_app_btn_" + (app.package_name || app.id), {
                width: px(touchW),
                height: px(touchH),
                background_color: hex("#00000000"),
                justify_content: "Center",
                align_items: "Center",
            }, [
                Container("dock_app_circle_" + app.id, {
                    width: px(iconSize),
                    height: px(iconSize),
                    background_color: hex("#CC1A2234"),
                    border_radius: iconRadius,
                    border_width: px(1.0),
                    border_color: hex("#5538BDF8"),
                    justify_content: "Center",
                    align_items: "Center",
                }, [
                    app.package_name ? Image("dock_img_" + app.id, "app-icon://" + app.package_name, {
                        width: px(iconSize * 0.88),
                        height: px(iconSize * 0.88),
                        border_radius: (iconSize * 0.88) / 2.0,
                        object_fit: "Cover"
                    }) : Container("dock_circle_" + app.id, {
                        width: px(iconSize * 0.88),
                        height: px(iconSize * 0.88),
                        border_radius: (iconSize * 0.88) / 2.0,
                        background_color: hex(app.color),
                        justify_content: "Center",
                        align_items: "Center",
                    }, [
                        Label("dock_app_icon_" + app.id, app.iconLetter, {
                            text_color: hex("#FFFFFF"),
                            text_size: vmin(3.8),
                            width: "Auto",
                        })
                    ])
                ])
            ]);
        }));
    }

    // Portrait Mode: Floating Translucent Android Dock Pill
    return Container("dock_root_container", {
        width: pct(100),
        height: px(vh(18.0)),
        background_color: hex("#00000000"),
        flex_direction: "Column",
        justify_content: "End",
        align_items: "Center",
        padding: {
            top: 0,
            bottom: vmin(2.2),
            left: vmin(2.0),
            right: vmin(2.0)
        },
    }, [
        Container("dock_apps_row_port", {
            width: pct(92),
            height: px(vmin(17.5)),
            background_color: hex("#B80F172A"),
            border_radius: vmin(8.75),
            border_width: px(1.0),
            border_color: hex("#33FFFFFF"),
            shadow_color: hex("#55000000"),
            shadow_offset_y: 4.0,
            shadow_spread: 6.0,
            flex_direction: "Row",
            justify_content: "SpaceAround",
            align_items: "Center",
            padding: padXY(0, vmin(1.0)),
        }, dockState.essentialApps.map(function(app) {
            const visualSize = vmin(15.0);
            const visualRadius = visualSize / 2.0;
            const touchW = vmin(17.0);
            const touchH = vmin(17.5);

            return Container("dock_app_btn_" + (app.package_name || app.id), {
                width: px(touchW),
                height: px(touchH),
                background_color: hex("#00000000"),
                justify_content: "Center",
                align_items: "Center",
            }, [
                Container("dock_app_circle_" + app.id, {
                    width: px(visualSize),
                    height: px(visualSize),
                    background_color: hex("#CC1A2234"),
                    border_radius: visualRadius,
                    border_width: px(1.0),
                    border_color: hex("#5538BDF8"),
                    justify_content: "Center",
                    align_items: "Center",
                }, [
                    app.package_name ? Image("dock_img_" + app.id, "app-icon://" + app.package_name, {
                        width: px(visualSize * 0.90),
                        height: px(visualSize * 0.90),
                        border_radius: (visualSize * 0.90) / 2.0,
                        object_fit: "Cover"
                    }) : Container("dock_circle_" + app.id, {
                        width: px(visualSize * 0.90),
                        height: px(visualSize * 0.90),
                        border_radius: (visualSize * 0.90) / 2.0,
                        background_color: hex(app.color),
                        justify_content: "Center",
                        align_items: "Center",
                    }, [
                        Label("dock_app_icon_" + app.id, app.iconLetter, {
                            text_color: hex("#FFFFFF"),
                            text_size: vmin(5.6),
                            width: "Auto",
                        })
                    ])
                ])
            ]);
        }))
    ]);
}
