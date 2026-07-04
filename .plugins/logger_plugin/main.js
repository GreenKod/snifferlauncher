// main.js - Plugin B (Logger)

host_log("[Logger Plugin] Started and registering APIs...");

// Kendi API'sini dışa açıyor:
registerApi("logger.logMessage", function(payload) {
    host_log("[Logger Plugin] Gelen İstek: " + JSON.stringify(payload));
    
    // İşlem yapılıp geri değer döndürülüyor:
    return { 
        success: true, 
        reply: "Mesaj başarıyla alındı! Zaman: " + payload.timestamp 
    };
});

globalThis.onEvent = function(eventJsonString) {
    return "[]";
};
