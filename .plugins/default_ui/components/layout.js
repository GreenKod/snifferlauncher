// ============================================================
// AppLayout - Dynamic Responsive Grid Layout System (Portrait & Landscape)
// ============================================================

function getGridConfig(isLandscape) {
    if (isLandscape) {
        // Landscape Mode: 7 Columns x 4 Rows (Left Grid, Right 14vmin Ergonomic Dock)
        const cols = 7;
        const rows = 4;
        const totalSlots = cols * rows; // 28

        const screenW = vw(100);
        const dockWPx = vmin(14.0);
        const availableGridWPx = Math.max(100, screenW - dockWPx);
        const gridWVw = (availableGridWPx / screenW) * 100.0;

        const cardW = 10.0;
        const cardH = 18.0;
        const gapX = 1.8;
        const gapY = 4.0;

        return {
            isLandscape: true,
            cols,
            rows,
            totalSlots,
            gridW: gridWVw,
            gridH: 100.0,
            cardW,
            cardH,
            gapX,
            gapY,
            taskManagerW: 14.0,
            taskManagerH: 100.0,
        };
    } else {
        // Portrait Mode: 4 Columns x 7 Rows (Top 86vh Grid, Bottom 14vh Dock)
        const cols = 4;
        const rows = 7;
        const totalSlots = cols * rows; // 28

        const cardW = 19.7;
        const cardH = 8.6;
        const gapX = 4.0;
        const gapY = 2.4;

        return {
            isLandscape: false,
            cols,
            rows,
            totalSlots,
            gridW: 100.0,
            gridH: 82.0,
            cardW,
            cardH,
            gapX,
            gapY,
            taskManagerW: 100.0,
            taskManagerH: 18.0,
        };
    }
}

const GRID_COLS = 4;
const GRID_ROWS = 7;
const TOTAL_APPS = GRID_COLS * GRID_ROWS; // 28

const GAP_X_VW = 4.0;
const CARD_W_VW = 19.7;

const GRID_HEIGHT_VH = 84.0;
const BOTTOM_TASKBAR_VH = 16.0;

const GAP_Y_VH = 1.8;
const CARD_H_VH = 9.8;

const GRID_COLORS = [
    [hex("#FF416C"), hex("#FF4B2B")],
    [hex("#1D2B64"), hex("#F8CDDA")],
    [hex("#6441A5"), hex("#2a0845")],
    [hex("#00B4DB"), hex("#0083B0")],
    [hex("#11998e"), hex("#38ef7d")],
    [hex("#FF8008"), hex("#FFC837")],
    [hex("#8E2DE2"), hex("#4A00E0")],
    [hex("#F5576C"), hex("#F093FB")],
    [hex("#00c6ff"), hex("#0072ff")],
    [hex("#f953c6"), hex("#b91d73")],
    [hex("#43e97b"), hex("#38f9d7")],
    [hex("#fa709a"), hex("#fee140")],
];
