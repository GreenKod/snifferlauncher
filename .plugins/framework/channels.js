// =============================================================================
// Channel Subscription Helper System
// =============================================================================

const _channelSubscribers = {};

function subscribeChannel(channelName, callback) {
    if (!_channelSubscribers[channelName]) {
        _channelSubscribers[channelName] = [];
    }
    _channelSubscribers[channelName].push(callback);
}

function unsubscribeChannel(channelName) {
    delete _channelSubscribers[channelName];
}

globalThis.onBroadcast = function(channel, data) {
    if (_channelSubscribers[channel]) {
        _channelSubscribers[channel].forEach(function(cb) {
            try { cb(data); } catch(e) {}
        });
    }
};
