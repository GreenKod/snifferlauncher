// App Grid Component — 4x7 Dynamic Grid Layout

function AppCardComponent(app, index, cfg) {
    const gradient = GRID_COLORS[index % GRID_COLORS.length];
    const letter = app.name ? app.name.charAt(0).toUpperCase() : "?";

    const cardW = vw(cfg ? cfg.cardW : CARD_W_VW);
    const cardH = vh(cfg ? cfg.cardH : CARD_H_VH);

    // İkonun / Kartın dikeyde esnemesini önlemek için kare bazlı min-size:
    const maxSide = Math.min(cardW, cardH);
    const iconSize = maxSide * 0.48;
    const iconRadius = iconSize / 2.0;
    const iconFontSize = iconSize * 0.50;
    const nameFontSize = Math.min(cardH * 0.16, cardW * 0.16);

    // Dinamik Metin Kısaltma Math Hesabı:
    // Kart genişliğinin %82'si metin alanı olarak ayrılır.
    // Karakter başına ortalama genişlik = font_size * 0.52
    const approxCharWidth = nameFontSize * 0.52;
    const maxAvailableWidth = cardW * 0.82;
    const maxChars = Math.max(3, Math.floor(maxAvailableWidth / approxCharWidth));

    let displayName = app.name || "";
    if (displayName.length > maxChars) {
        displayName = displayName.substring(0, Math.max(1, maxChars - 2)) + "..";
    }

    return Container("card_" + index, {
        width: px(cardW),
        height: px(cardH),
        background_color: hex("#15151F"),
        border_radius: vmin(1.2),
        flex_direction: "Column",
        align_items: "Center",
        justify_content: "Center",
        gap: cardH * 0.04,
        shadow_color: hex("#33000000"),
        shadow_offset_y: vmin(0.2),
        shadow_spread: vmin(0.3),
    }, [
        Container("icon_" + index, {
            width: px(iconSize),
            height: px(iconSize),
            border_radius: iconRadius,
            background_gradient: gradient,
            justify_content: "Center",
            align_items: "Center",
        }, [
            Label("letter_" + index, letter, {
                text_color: hex("#FFFFFF"),
                text_size: iconFontSize,
                width: "Auto",
            })
        ]),
        Label("name_" + index, displayName, {
            text_color: hex("#DDDDDD"),
            text_size: nameFontSize,
            width: "Auto",
        })
    ]);
}

function EmptySlotComponent(index, cfg) {
    const cardW = vw(cfg ? cfg.cardW : CARD_W_VW);
    const cardH = vh(cfg ? cfg.cardH : CARD_H_VH);

    return Container("empty_slot_" + index, {
        width: px(cardW),
        height: px(cardH),
        background_color: hex("#00000000"), // Rezerve edilmiş şeffaf boş alan
    }, []);
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

    const pageApps = state.apps; // Seçili sayfadaki (maksimum 28) uygulamalar

    const gridItems = [];
    for (let index = 0; index < cfg.totalSlots; index++) {
        if (index < pageApps.length) {
            gridItems.push(AppCardComponent(pageApps[index], index, cfg));
        } else {
            gridItems.push(EmptySlotComponent(index, cfg));
        }
    }

    return [
        Container("app_grid", {
            width: px(vw(cfg.gridW)),
            height: px(vh(cfg.gridH)),
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
        }, gridItems)
    ];
}