// Dock Bar Virtual DOM Generator
function getDockContainer(isLandscape) {
    if (isLandscape) {
        // Landscape Mode: Right Vertical Ergonomic Dock (vmin scaled)
        return Container("dock_root_container", {
            width: px(vmin(14.0)),
            height: pct(100),
            background_color: hex("#00000000"),
            flex_direction: "Column",
            justify_content: "Center",
            align_items: "Center",
            padding: padXY(vmin(1.0), vmin(1.0)),
            transition: {
                duration: 0.35,
                easing: {
                    spring: {
                        stiffness: 280.0,
                        damping: 24.0
                    }
                }
            }
        }, [
            Container("dock_bar_pill", {
                width: px(vmin(12.5)),
                height: px(vmin(68.0)),
                background_color: hex("#400F172A"),
                backdrop_blur: 24.0,
                backdrop_tint: hex("#800F172A"),
                elevation: 8.0,
                shadow_blur: 16.0,
                shadow_spread: 2.0,
                shadow_offset_y: 4.0,
                shadow_color: hex("#40000000"),
                border_radius: vmin(6.25),
                border_width: px(1.0),
                border_color: hex("#33FFFFFF"),
                border_gradient: [hex("#66FFFFFF"), hex("#15FFFFFF")],
                flex_direction: "Column",
                justify_content: "Center",
                align_items: "Center",
                padding: padXY(vmin(1.0), vmin(1.5)),
                gap: vmin(2.5),
                transition: {
                    duration: 0.35,
                    easing: {
                        spring: {
                            stiffness: 280.0,
                            damping: 24.0
                        }
                    }
                }
            }, dockState.essentialApps.map(function(app) {
                const iconSize = vmin(9.5);
                const iconRadius = iconSize / 2.0;
                const touchW = vmin(11.5);
                const touchH = vmin(11.5);
                const isPressed = dockState.pressedAppId === app.id;
                
                return Container("dock_app_btn_" + (app.package_name || app.id), {
                    width: px(touchW),
                    height: px(touchH),
                    background_color: hex("#00000000"),
                    justify_content: "Center",
                    align_items: "Center",
                    transition: {
                        duration: 0.35,
                        easing: {
                            spring: {
                                stiffness: 280.0,
                                damping: 24.0
                            }
                        }
                    }
                }, [
                    Container("dock_app_circle_" + app.id, {
                        width: px(iconSize),
                        height: px(iconSize),
                        background_color: hex("#CC1A2234"),
                        border_radius: iconRadius,
                        border_width: px(1.0),
                        border_color: hex("#5538BDF8"),
                        border_gradient: [hex("#6638BDF8"), hex("#1A38BDF8")],
                        elevation: isPressed ? 2.0 : 6.0,
                        shadow_blur: isPressed ? 4.0 : 10.0,
                        shadow_spread: isPressed ? 0.5 : 1.0,
                        shadow_offset_y: isPressed ? 1.0 : 3.0,
                        shadow_color: hex("#40000000"),
                        transform: {
                            scale: isPressed ? 0.88 : 1.0
                        },
                        transition: {
                            duration: 0.22,
                            easing: {
                                spring: {
                                    stiffness: 320.0,
                                    damping: 22.0
                                }
                            }
                        },
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
            }))
        ]);
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
        transition: {
            duration: 0.35,
            easing: {
                spring: {
                    stiffness: 280.0,
                    damping: 24.0
                }
            }
        }
    }, [
        Container("dock_bar_pill", {
            width: pct(92),
            height: px(vmin(17.5)),
            background_color: hex("#400F172A"),
            backdrop_blur: 24.0,
            backdrop_tint: hex("#800F172A"),
            elevation: 8.0,
            shadow_blur: 16.0,
            shadow_spread: 2.0,
            shadow_offset_y: 4.0,
            shadow_color: hex("#40000000"),
            border_radius: vmin(8.75),
            border_width: px(1.0),
            border_color: hex("#33FFFFFF"),
            border_gradient: [hex("#66FFFFFF"), hex("#15FFFFFF")],
            flex_direction: "Row",
            justify_content: "SpaceAround",
            align_items: "Center",
            padding: padXY(0, vmin(1.0)),
            transition: {
                duration: 0.35,
                easing: {
                    spring: {
                        stiffness: 280.0,
                        damping: 24.0
                    }
                }
            }
        }, dockState.essentialApps.map(function(app) {
            const visualSize = vmin(15.0);
            const visualRadius = visualSize / 2.0;
            const touchW = vmin(17.0);
            const touchH = vmin(17.5);
            const isPressed = dockState.pressedAppId === app.id;

            return Container("dock_app_btn_" + (app.package_name || app.id), {
                width: px(touchW),
                height: px(touchH),
                background_color: hex("#00000000"),
                justify_content: "Center",
                align_items: "Center",
                transition: {
                    duration: 0.35,
                    easing: {
                        spring: {
                            stiffness: 280.0,
                            damping: 24.0
                        }
                    }
                }
            }, [
                Container("dock_app_circle_" + app.id, {
                    width: px(visualSize),
                    height: px(visualSize),
                    background_color: hex("#CC1A2234"),
                    border_radius: visualRadius,
                    border_width: px(1.0),
                    border_color: hex("#5538BDF8"),
                    border_gradient: [hex("#6638BDF8"), hex("#1A38BDF8")],
                    elevation: isPressed ? 2.0 : 6.0,
                    shadow_blur: isPressed ? 4.0 : 10.0,
                    shadow_spread: isPressed ? 0.5 : 1.0,
                    shadow_offset_y: isPressed ? 1.0 : 3.0,
                    shadow_color: hex("#40000000"),
                    transform: {
                        scale: isPressed ? 0.88 : 1.0
                    },
                    transition: {
                        duration: 0.22,
                        easing: {
                            spring: {
                                stiffness: 320.0,
                                damping: 22.0
                            }
                        }
                    },
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
