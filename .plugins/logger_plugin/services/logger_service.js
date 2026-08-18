// Logger Service

function logFormattedMessage(payload) {
    const timestamp = getLocalTime().time;
    const level = payload.level || "INFO";
    const msg = payload.message || "";
    
    host_log(`[LOG - ${level} @ ${timestamp}]: ${msg}`);
    return { success: true, loggedAt: timestamp };
}
