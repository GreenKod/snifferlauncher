// Clock Widget - UI Component

function renderClockCard() {
    const now = getLocalTime();
    const offset = defaultSettings.utcOffset ?? 3;

    return CardWidget({
        id: "clock-widget-card",
        title: "CLOCK & DATE",
        subtitle: `Local Time (UTC+${offset})`,
        content: [
            Label("clock-card-time", now.time, {
                text_color: Theme.colors.accent,
                text_size: vmin(6.5),
                width: "Auto"
            }),
            Label("clock-card-date", now.date, {
                text_color: Theme.colors.textSub,
                text_size: vmin(3.5),
                width: "Auto"
            })
        ]
    });
}
