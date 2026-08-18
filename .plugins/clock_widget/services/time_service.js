// Clock Widget - Time Service

function getTimeData(offset) {
    const now = getLocalTime();
    return {
        success: true,
        time: now.time,
        date: now.date
    };
}
