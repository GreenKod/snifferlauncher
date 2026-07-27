// Logger Plugin Main Entry

host_log("[Logger Plugin] Started and registering APIs...");

registerApi("logger.logMessage", function(payload) {
    return logFormattedMessage(payload);
});
