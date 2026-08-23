// =============================================================================
// SnifferLauncher Inter-Plugin Communication (IPC) & Permissions
// =============================================================================

globalThis._apis = {};

globalThis.registerApi = function(name, handler) {
    globalThis._apis[name] = handler;
    host_register_api(name);
};

globalThis.callApi = function(name, payload) {
    const json = host_call_api(name, JSON.stringify(payload !== undefined && payload !== null ? payload : {}));
    if (json === null || json === undefined) return null;
    try { return JSON.parse(json); } catch(e) { return null; }
};

globalThis.broadcastEvent = function(channel, data) {
    host_broadcast(channel, JSON.stringify(data !== undefined && data !== null ? data : {}));
};

globalThis.requestPermissions = function(permissions) {
    return JSON.parse(host_request_permissions(permissions));
};

globalThis.hasPermission = function(permission) {
    return host_has_permission(permission);
};
