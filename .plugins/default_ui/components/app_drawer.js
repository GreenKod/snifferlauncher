// =============================================================================
// AppDrawerComponent - "Alt Kat" (Uygulama Çekmecesi)
// En üstte SearchBar, altında A-Z alfabetik dizilmiş dikey kaydırılabilir liste
// =============================================================================

function AppDrawerCardComponent(app, index, isLandscape) {
    const letter = app.name ? app.name.charAt(0).toUpperCase() : "?";
    const gradient = GRID_COLORS[index % GRID_COLORS.length];

    const cardW = isLandscape ? vw(13.0) : vw(21.5);
    const cardH = isLandscape ? vh(18.0) : vh(10.5);

    const maxSide = Math.min(cardW, cardH);
    const iconSize = maxSide * 0.48;
    const iconRadius = iconSize / 2.0;
    const iconFontSize = iconSize * 0.50;
    const nameFontSize = Math.min(cardH * 0.16, cardW * 0.16);

    const approxCharWidth = nameFontSize * 0.52;
    const maxAvailableWidth = cardW * 0.85;
    const maxChars = Math.max(3, Math.floor(maxAvailableWidth / approxCharWidth));

    let displayName = app.name || "";
    if (displayName.length > maxChars) {
        displayName = displayName.substring(0, Math.max(1, maxChars - 2)) + "..";
    }

    const uniqueId = "drawer_card_" + index;

    return Container(uniqueId, {
        width: px(cardW),
        height: px(cardH),
        background_color: hex("#E0151B27"),
        border_radius: vmin(1.4),
        border_width: 1.0,
        border_color: hex("#20FFFFFF"),
        flex_direction: "Column",
        align_items: "Center",
        justify_content: "Center",
        gap: cardH * 0.05,
        transition: { duration: 0.20 },
    }, [
        Container("drawer_icon_" + index, {
            width: px(iconSize),
            height: px(iconSize),
            border_radius: iconRadius,
            background_gradient: gradient,
            justify_content: "Center",
            align_items: "Center",
        }, [
            app.package_name ? Image("drawer_img_" + index, "app-icon://" + app.package_name, {
                width: px(iconSize),
                height: px(iconSize),
                border_radius: iconRadius,
                object_fit: "Cover"
            }) : Label("drawer_letter_" + index, letter, {
                text_color: hex("#FFFFFF"),
                text_size: iconFontSize,
                width: "Auto",
            })
        ]),
        Label("drawer_name_" + index, displayName, {
            text_color: hex("#ECEFF4"),
            text_size: nameFontSize,
            width: "Auto",
        })
    ]);
}

function AppDrawerComponent() {
    const isLandscape = vw(100) > vh(100);
    const isBottomFloor = state.currentFloor === "bottom";
    const apps = state.filteredDrawerApps;
    const query = state.drawerSearchQuery || "";

    const cardItems = [];
    for (let i = 0; i < apps.length; i++) {
        cardItems.push(AppDrawerCardComponent(apps[i], i, isLandscape));
    }

    // SearchBar Container (Sleek Modern Frosted Glass Pill)
    const searchBarContainer = Container("drawer_search_container", {
        width: pct(100),
        height: px(vh(5.2)),
        background_color: hex("#EE161B26"),
        border_radius: vmin(2.6),
        border_width: 1.0,
        border_color: hex("#5538BDF8"),
        flex_direction: "Row",
        align_items: "Center",
        justify_content: "SpaceBetween",
        padding: padXY(0, vw(3.5)),
        gap: vw(2.0),
        transition: { duration: 0.2 },
    }, [
        // Left & middle part: Icon + Full-width TextInput
        Container("drawer_search_left_group", {
            flex_grow: 1.0,
            height: pct(100),
            flex_direction: "Row",
            align_items: "Center",
            gap: vw(2.0),
        }, [
            Label("drawer_search_icon", "🔍", {
                text_color: hex("#94A3B8"),
                text_size: vmin(3.6),
                width: "Auto",
            }),
            TextInput("drawer_search_input", query, {
                placeholder: "Uygulamalarda ara (A-Z)...",
                focused: !!state.isSearchFocused,
                style: {
                    flex_grow: 1.0,
                    width: pct(100),
                    height: px(vh(4.6)),
                    background_color: hex("#00000000"),
                    text_color: hex("#F8FAFC"),
                    text_size: vmin(3.6),
                }
            })
        ]),
        // Right part: Clear button pinned at the far right
        query ? Container("drawer_search_clear", {
            width: px(vmin(5.6)),
            height: px(vmin(5.6)),
            background_color: hex("#334155"),
            border_radius: vmin(2.8),
            justify_content: "Center",
            align_items: "Center",
        }, [
            Label("drawer_clear_label", "✕", {
                text_color: hex("#CBD5E1"),
                text_size: vmin(3.0),
                width: "Auto",
            })
        ]) : Container("drawer_search_spacer", { width: px(0), height: px(0) }, [])
    ]);

    const drawerChildren = [
        // Drawer Handle Pill (Android tarzı üst çekme çizgisi)
        Container("drawer_handle_pill_wrapper", {
            width: pct(100),
            height: px(vh(6.5)),
            padding: padTRBL(vh(3.8), 0, 0, 0),
            justify_content: "Center",
            align_items: "Center",
        }, [
            Container("drawer_handle", {
                width: px(vmin(10.0)),
                height: px(vmin(0.6)),
                background_color: hex("#80FFFFFF"),
                border_radius: vmin(0.3),
            }, [])
        ]),

        // Top Bar (Sleek SearchBar)
        Container("drawer_header", {
            width: pct(100),
            height: px(vh(6.6)),
            flex_direction: "Row",
            align_items: "Center",
            justify_content: "Center",
            padding: padXY(vh(0.4), vw(3.5)),
        }, [searchBarContainer])
    ];

    if (cardItems.length === 0) {
        drawerChildren.push(
            Container("drawer_empty", {
                width: pct(100),
                height: px(vh(70.0)),
                justify_content: "Center",
                align_items: "Center",
                flex_direction: "Column",
                gap: vmin(1.5),
            }, [
                Label("drawer_empty_icon", "🔍", {
                    text_color: hex("#64748B"),
                    text_size: vmin(6.0),
                    width: "Auto",
                }),
                Label("drawer_empty_text", "Uygulama bulunamadı", {
                    text_color: hex("#94A3B8"),
                    text_size: vmin(2.2),
                    width: "Auto",
                })
            ])
        );
    } else {
        const gapX = isLandscape ? vw(1.8) : vw(2.2);
        const gapY = isLandscape ? vh(2.0) : vh(1.8);

        drawerChildren.push(
            ScrollView("app_drawer_scroll", {
                dynamic_sensitivity: true,
                momentum_scrolling: true,
                style: {
                    width: pct(100),
                    height: px(vh(81.5)),
                    overflow_hidden: true,
                }
            }, [
                Container("drawer_grid_wrapper", {
                    width: pct(100),
                    height: "Auto",
                    flex_direction: "Row",
                    flex_wrap: "Wrap",
                    justify_content: "Start",
                    align_content: "Start",
                    padding: padXY(vh(1.5), vw(3.0)),
                    column_gap: gapX,
                    row_gap: gapY,
                }, cardItems)
            ])
        );
    }

    return Container("app_drawer_root", {
        position: "Absolute",
        top: px(0),
        left: px(0),
        width: pct(100),
        height: pct(100),
        background_gradient: [hex("#FF0A0E18"), hex("#FF05070D")],
        flex_direction: "Column",
        justify_content: "Start",
        align_items: "Stretch",
        opacity: isBottomFloor ? 1.0 : 0.0,
        transform: {
            translate_y: isBottomFloor ? 0.0 : vh(100.0),
            scale: 1.0,
        },
        transition: {
            duration: 0.32,
            easing: "ease_out"
        }
    }, drawerChildren);
}
