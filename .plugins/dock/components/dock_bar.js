// Dock Bar Virtual DOM Generator
function renderBadge(app, isLandscape) {
    if (!dockState.badges || !dockState.badges[app.id]) {
        return null;
    }
    const badge = dockState.badges[app.id];
    const isCounter = typeof badge.count === "number" && badge.count > 0;

    if (isCounter) {
        const countText = badge.count > 99 ? "99+" : String(badge.count);
        return Container("dock_badge_counter_" + app.id, {
            position: "Absolute",
            top: px(vmin(0.4)),
            right: px(vmin(0.4)),
            min_width: px(isLandscape ? vmin(3.6) : vmin(4.8)),
            height: px(isLandscape ? vmin(3.2) : vmin(4.2)),
            border_radius: isLandscape ? vmin(1.6) : vmin(2.1),
            background_color: hex("#EF4444"),
            border_width: px(1.5),
            border_color: hex("#0F172A"),
            elevation: 4.0,
            shadow_blur: 6.0,
            shadow_color: hex("#60EF4444"),
            justify_content: "Center",
            align_items: "Center",
            padding: padXY(0, isLandscape ? vmin(0.6) : vmin(1.0)),
            transition: {
                duration: 0.22,
                easing: {
                    spring: {
                        stiffness: 320.0,
                        damping: 22.0
                    }
                }
            }
        }, [
            Label("dock_badge_label_" + app.id, countText, {
                text_color: hex("#FFFFFF"),
                text_size: isLandscape ? vmin(2.0) : vmin(2.8),
                font_weight: "Bold",
                width: "Auto",
            })
        ]);
    }

    // Dot Badge
    return Container("dock_badge_dot_" + app.id, {
        position: "Absolute",
        top: px(vmin(0.6)),
        right: px(vmin(0.6)),
        width: px(isLandscape ? vmin(2.4) : vmin(3.2)),
        height: px(isLandscape ? vmin(2.4) : vmin(3.2)),
        border_radius: isLandscape ? vmin(1.2) : vmin(1.6),
        background_color: hex("#EF4444"),
        border_width: px(1.5),
        border_color: hex("#0F172A"),
        elevation: 4.0,
        shadow_blur: 6.0,
        shadow_color: hex("#60EF4444"),
        transition: {
            duration: 0.22,
            easing: {
                spring: {
                    stiffness: 320.0,
                    damping: 22.0
                }
            }
        }
    });
}

function renderPreviewBubble(isLandscape) {
    if (!dockState.activePreviewAppId) {
        return null;
    }
    const app = (dockState.essentialApps || []).find(function(a) {
        return a.id === dockState.activePreviewAppId;
    });
    if (!app) {
        return null;
    }

    const previewIconSize = isLandscape ? vmin(7.0) : vmin(8.5);

    return Container("dock_preview_bubble", {
        position: "Absolute",
        bottom: isLandscape ? "Auto" : px(vmin(20.5)),
        right: isLandscape ? px(vmin(14.5)) : "Auto",
        background_color: hex("#E61E293B"),
        backdrop_blur: 20.0,
        backdrop_tint: hex("#D90F172A"),
        elevation: 12.0,
        shadow_blur: 24.0,
        shadow_spread: 2.0,
        shadow_offset_y: 6.0,
        shadow_color: hex("#80000000"),
        border_radius: vmin(4.0),
        border_width: px(1.0),
        border_color: hex("#40FFFFFF"),
        border_gradient: [hex("#80FFFFFF"), hex("#20FFFFFF")],
        padding: padXY(vmin(1.2), vmin(2.2)),
        flex_direction: "Row",
        align_items: "Center",
        gap: vmin(2.0),
        transform: {
            scale: 1.0
        },
        transition: {
            duration: 0.25,
            easing: {
                spring: {
                    stiffness: 320.0,
                    damping: 24.0
                }
            }
        }
    }, [
        app.package_name ? Image("dock_preview_img", "app-icon://" + app.package_name, {
            width: px(previewIconSize),
            height: px(previewIconSize),
            border_radius: previewIconSize / 2.0,
            object_fit: "Cover"
        }) : Container("dock_preview_circle", {
            width: px(previewIconSize),
            height: px(previewIconSize),
            border_radius: previewIconSize / 2.0,
            background_color: hex(app.color),
            justify_content: "Center",
            align_items: "Center",
        }, [
            Label("dock_preview_letter", app.iconLetter, {
                text_color: hex("#FFFFFF"),
                text_size: previewIconSize * 0.45,
                width: "Auto",
            })
        ]),
        Container("dock_preview_texts", {
            flex_direction: "Column",
            justify_content: "Center",
            gap: vmin(0.4),
        }, [
            Label("dock_preview_title", app.name, {
                text_color: hex("#FFFFFF"),
                text_size: isLandscape ? vmin(2.8) : vmin(3.4),
                font_weight: "Bold",
                width: "Auto",
            }),
            Label("dock_preview_action", "Hızlı Başlat • Bilgi", {
                text_color: hex("#94A3B8"),
                text_size: isLandscape ? vmin(2.0) : vmin(2.4),
                width: "Auto",
            })
        ]),
        Container("dock_preview_close_btn", {
            width: px(vmin(5.5)),
            height: px(vmin(5.5)),
            border_radius: vmin(2.75),
            background_color: hex("#25FFFFFF"),
            justify_content: "Center",
            align_items: "Center",
        }, [
            Label("dock_preview_close_txt", "✕", {
                text_color: hex("#FFFFFF"),
                text_size: vmin(2.6),
                width: "Auto",
            })
        ])
    ]);
}

function getDockContainer(isLandscape) {
    const preview = renderPreviewBubble(isLandscape);

    if (isLandscape) {
        // Landscape Mode: Right Vertical Ergonomic Dock (vmin scaled)
        const rootChildren = [
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
                const badgeElement = renderBadge(app, true);
                
                const btnChildren = [
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
                ];

                if (badgeElement) {
                    btnChildren.push(badgeElement);
                }

                return Container("dock_app_btn_" + (app.package_name || app.id), {
                    width: px(touchW),
                    height: px(touchH),
                    position: "Relative",
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
                }, btnChildren);
            }))
        ];

        if (preview) {
            rootChildren.push(preview);
        }

        return Container("dock_root_container", {
            width: px(vmin(14.0)),
            height: pct(100),
            position: "Relative",
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
        }, rootChildren);
    }

    // Portrait Mode: Floating Translucent Android Dock Pill
    const portraitPill = Container("dock_bar_pill", {
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
        const badgeElement = renderBadge(app, false);

        const btnChildren = [
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
        ];

        if (badgeElement) {
            btnChildren.push(badgeElement);
        }

        return Container("dock_app_btn_" + (app.package_name || app.id), {
            width: px(touchW),
            height: px(touchH),
            position: "Relative",
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
        }, btnChildren);
    }));

    const rootChildren = [portraitPill];
    if (preview) {
        rootChildren.push(preview);
    }

    return Container("dock_root_container", {
        width: pct(100),
        height: px(vh(18.0)),
        position: "Relative",
        background_color: hex("#00000000"),
        flex_direction: "Column",
        justify_content: "Start",
        align_items: "Center",
        padding: {
            top: vmin(2.0),
            bottom: vmin(6.5),
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
    }, rootChildren);
}
