// =============================================================================
// Broadcast Listeners, Handlers & SharedView Handshake Protocol
// =============================================================================

globalThis._broadcastListeners = {};

globalThis.subscribeChannel = function(channel, callback) {
    if (typeof callback !== "function") return;
    if (!globalThis._broadcastListeners[channel]) {
        globalThis._broadcastListeners[channel] = [];
    }
    globalThis._broadcastListeners[channel].push(callback);
};

globalThis.unsubscribeChannel = function(channel) {
    delete globalThis._broadcastListeners[channel];
};

globalThis.onBroadcast = function(channelOrCallback, callback) {
    if (typeof channelOrCallback === "function") {
        globalThis.subscribeChannel("*", channelOrCallback);
    } else if (typeof channelOrCallback === "string" && typeof callback === "function") {
        globalThis.subscribeChannel(channelOrCallback, callback);
    }
};

globalThis._sharedViewPending = {};

globalThis.requestSharedView = function(targetPluginId, slotName, payload, timeoutMs) {
    const tMs = (typeof timeoutMs === 'number' && timeoutMs > 0) ? timeoutMs : 3000;
    return new Promise(function(resolve) {
        const invitationId = "inv_" + Math.random().toString(36).substring(2, 10);
        
        const timer = setTimeout(function() {
            if (globalThis._sharedViewPending[invitationId]) {
                delete globalThis._sharedViewPending[invitationId];
                resolve({
                    accepted: false,
                    status: "timed_out",
                    reason: "Response timeout exceeded (" + tMs + "ms)"
                });
            }
        }, tMs);

        globalThis._sharedViewPending[invitationId] = { resolve: resolve, timer: timer };

        const payloadStr = JSON.stringify(payload !== undefined && payload !== null ? payload : {});
        broadcastEvent("shared_view_invite", {
            invitationId: invitationId,
            targetPluginId: targetPluginId,
            slotName: slotName,
            payload: payloadStr,
            timeoutMs: tMs
        });
    });
};

globalThis.acceptSharedView = function(invitationId, uiTree) {
    broadcastEvent("shared_view_response", {
        invitationId: invitationId,
        accepted: true,
        status: "accepted",
        uiTree: uiTree !== undefined ? uiTree : null
    });
};

globalThis.rejectSharedView = function(invitationId, reason) {
    broadcastEvent("shared_view_response", {
        invitationId: invitationId,
        accepted: false,
        status: "rejected",
        reason: reason !== undefined && reason !== null ? reason : "Invitation rejected"
    });
};

globalThis.onRequestSharedView = function(invitation) {
    globalThis.rejectSharedView(invitation.invitationId, "No handler registered");
};

globalThis._dispatchBroadcast = function(channel, payload_json) {
    try {
        const data = JSON.parse(payload_json);
        
        if (channel === "shared_view_invite") {
            if (data.targetPluginId && typeof globalThis.onRequestSharedView === 'function') {
                globalThis.onRequestSharedView({
                    invitationId: data.invitationId,
                    slotName: data.slotName,
                    payload: JSON.parse(data.payload !== undefined && data.payload !== null ? data.payload : "{}")
                });
            }
        } else if (channel === "shared_view_response") {
            const pending = globalThis._sharedViewPending[data.invitationId];
            if (pending) {
                clearTimeout(pending.timer);
                delete globalThis._sharedViewPending[data.invitationId];
                pending.resolve({
                    accepted: !!data.accepted,
                    status: data.status,
                    reason: data.reason,
                    uiTree: data.uiTree
                });
            }
        }

        if (globalThis._broadcastListeners[channel]) {
            globalThis._broadcastListeners[channel].forEach(function(cb) {
                try { cb(data); } catch(e) {}
            });
        }

        if (globalThis._broadcastListeners["*"]) {
            globalThis._broadcastListeners["*"].forEach(function(cb) {
                try { cb(channel, data); } catch(e) {}
            });
        }
    } catch(e) {}
};

globalThis._handleApiCall = function(name, payload_json) {
    if (!globalThis._apis[name]) return null;
    try {
        const payload = JSON.parse(payload_json);
        const result = globalThis._apis[name](payload);
        return JSON.stringify(result !== undefined ? result : null);
    } catch(e) {
        return null;
    }
};
