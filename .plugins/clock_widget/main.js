// Clock Widget - Main Entry Point

host_log("[Clock Widget] Started and registering APIs...");

registerApi("clock.getTime", function(payload) {
    host_log("[Clock Widget] Gelen İstek: " + JSON.stringify(payload));
    return getTimeData(payload?.utcOffset);
});

registerApi("clock.getWidgetUI", function(payload) {
    return renderClockCard();
});

// Broadcast clock.secondChanged every 1000ms
setInterval(function() {
    const now = getLocalTime();
    broadcastEvent("clock.secondChanged", { second: now.seconds, time: now.time });
}, 1000);