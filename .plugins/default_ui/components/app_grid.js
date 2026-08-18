function AppCardComponent(app, pageIndex, index, cfg) {
    const gradient = GRID_COLORS[(pageIndex * cfg.totalSlots + index) % GRID_COLORS.length];
    const letter = app.name ? app.name.charAt(0).toUpperCase() : "?";

    const cardW = vw(cfg ? cfg.cardW : CARD_W_VW);
    const cardH = vh(cfg ? cfg.cardH : CARD_H_VH);

    const maxSide = Math.min(cardW, cardH);
    const iconSize = maxSide * 0.48;
    const iconRadius = iconSize / 2.0;
    const iconFontSize = iconSize * 0.50;
    const nameFontSize = Math.min(cardH * 0.16, cardW * 0.16);

    const approxCharWidth = nameFontSize * 0.52;
    const maxAvailableWidth = cardW * 0.82;
    const maxChars = Math.max(3, Math.floor(maxAvailableWidth / approxCharWidth));

    let displayName = app.name || "";
    if (displayName.length > maxChars) {
        displayName = displayName.substring(0, Math.max(1, maxChars - 2)) + "..";
    }

    const uniqueId = "p" + pageIndex + "_card_" + index;

    return Container(uniqueId, {
        width: px(cardW),
        height: px(cardH),
        background_color: hex("#15151F"),
        border_radius: vmin(1.2),
        flex_direction: "Column",
        align_items: "Center",
        justify_content: "Center",
        gap: cardH * 0.04,
    }, [
        Container("p" + pageIndex + "_icon_" + index, {
            width: px(iconSize),
            height: px(iconSize),
            border_radius: iconRadius,
            background_gradient: gradient,
            justify_content: "Center",
            align_items: "Center",
        }, [
            app.package_name ? Image("p" + pageIndex + "_img_" + index, "app-icon://" + app.package_name, {
                width: px(iconSize),
                height: px(iconSize),
                border_radius: iconRadius,
                object_fit: "Cover"
            }) : Label("p" + pageIndex + "_letter_" + index, letter, {
                text_color: hex("#FFFFFF"),
                text_size: iconFontSize,
                width: "Auto",
            })
        ]),
        Label("p" + pageIndex + "_name_" + index, displayName, {
            text_color: hex("#DDDDDD"),
            text_size: nameFontSize,
            width: "Auto",
        })
    ]);
}

function EmptySlotComponent(pageIndex, index, cfg) {
    const cardW = vw(cfg ? cfg.cardW : CARD_W_VW);
    const cardH = vh(cfg ? cfg.cardH : CARD_H_VH);

    return Container("p" + pageIndex + "_empty_slot_" + index, {
        width: px(cardW),
        height: px(cardH),
    }, []);
}

function SinglePageGridComponent(pageIndex, cfg) {
    const pageApps = state.appsForPage(pageIndex);
    const gridItems = [];

    for (let index = 0; index < cfg.totalSlots; index++) {
        if (index < pageApps.length) {
            gridItems.push(AppCardComponent(pageApps[index], pageIndex, index, cfg));
        } else {
            gridItems.push(EmptySlotComponent(pageIndex, index, cfg));
        }
    }

    return Container("page_grid_" + pageIndex, {
        width: px(vw(cfg.gridW)),
        height: px(vh(cfg.gridH)),
        flex_shrink: 0,
        flex_direction: "Row",
        flex_wrap: "Wrap",
        justify_content: "Start",
        align_content: "Start",
        padding: padXY(
            vh(cfg.gapY),
            vw(cfg.gapX)
        ),
        column_gap: vw(cfg.gapX),
        row_gap: vh(cfg.gapY),
    }, gridItems);
}

function PageIndicatorDots(totalPages, currentPage, isLandscape) {
    if (totalPages <= 1) return null;

    const dots = [];
    for (let p = 0; p < totalPages; p++) {
        const isActive = p === currentPage;
        if (isLandscape) {
            // Yatay Ekranda: Sağ tarafta Dikey Gösterge Noktaları
            dots.push(Container("page_dot_" + p, {
                width: px(vmin(1.6)),
                height: px(isActive ? vmin(3.6) : vmin(1.6)),
                border_radius: vmin(0.8),
                background_color: hex(isActive ? "#00E5FF" : "#ffffff44"),
                transition: { duration: 0.2 },
            }, []));
        } else {
            // Dikey Ekranda: Alt tarafta Yatay Gösterge Noktaları
            dots.push(Container("page_dot_" + p, {
                width: px(isActive ? vmin(3.6) : vmin(1.6)),
                height: px(vmin(1.6)),
                border_radius: vmin(0.8),
                background_color: hex(isActive ? "#00E5FF" : "#ffffff44"),
                transition: { duration: 0.2 },
            }, []));
        }
    }

    if (isLandscape) {
        return Container("page_indicator_container", {
            position: "Absolute",
            right: px(vmin(1.2)),
            top: px(0.0),
            height: pct(100),
            width: "Auto",
            flex_direction: "Column",
            justify_content: "Center",
            align_items: "Center",
            gap: vmin(1.2),
        }, dots);
    } else {
        return Container("page_indicator_container", {
            position: "Absolute",
            bottom: px(vmin(1.5)),
            width: pct(100),
            height: "Auto",
            flex_direction: "Row",
            justify_content: "Center",
            align_items: "Center",
            gap: vmin(1.2),
        }, dots);
    }
}

function getPageGrids(totalPages, cfg, isLandscape) {
    const grids = [];
    for (let p = 0; p < totalPages; p++) {
        grids.push(SinglePageGridComponent(p, cfg));
    }
    return grids;
}

function AppGridComponent() {
    const isLandscape = vw(100) > vh(100);
    const cfg = typeof getGridConfig === "function"
        ? getGridConfig(isLandscape)
        : {
            totalSlots: TOTAL_APPS,
            gridW: 100.0,
            gridH: GRID_HEIGHT_VH,
            cardW: CARD_W_VW,
            cardH: CARD_H_VH,
            gapX: GAP_X_VW,
            gapY: GAP_Y_VH,
        };

    const totalPages = state.totalPages;
    const pageWidthPx = vw(cfg.gridW);
    const trackWidthPx = pageWidthPx * totalPages;

    const pageGrids = getPageGrids(totalPages, cfg, isLandscape);
    const indicator = PageIndicatorDots(totalPages, state.currentPage, isLandscape);

    const wrapperChildren = [
        ScrollView("app_grid_pager", {
            snap_x: pageWidthPx,
            rubber_band: 0.20,
            page_count: totalPages,
            on_snap: "onPageChanged",
            momentum_scrolling: false,
            style: {
                width: px(pageWidthPx),
                height: px(vh(cfg.gridH)),
                overflow_hidden: true,
            }
        }, [
            Container("app_grid_track", {
                width: px(trackWidthPx),
                height: px(vh(cfg.gridH)),
                flex_shrink: 0,
                flex_direction: "Row",
            }, pageGrids)
        ])
    ];

    if (indicator) {
        wrapperChildren.push(indicator);
    }

    return Container("app_grid_wrapper", {
        width: px(pageWidthPx),
        height: px(vh(cfg.gridH)),
        position: "Relative",
    }, wrapperChildren);
}