// =============================================================================
// Timers Polyfill (setInterval, setTimeout, clearInterval, clearTimeout)
// =============================================================================

globalThis._timers = {};
globalThis._timerId = 1;

function _hasActiveTimers() {
    for (const _ in globalThis._timers) {
        return true;
    }
    return false;
}

function _updateActiveTimers() {
    if (typeof host_set_active_timers === "function") {
        host_set_active_timers(_hasActiveTimers());
    }
}

globalThis.setInterval = function(callback, delayMs) {
    const id = globalThis._timerId++;
    globalThis._timers[id] = {
        callback: callback,
        delay: (typeof delayMs === 'number' && delayMs >= 0) ? delayMs : 1000,
        lastRun: Date.now(),
        once: false
    };
    _updateActiveTimers();
    return id;
};

globalThis.clearInterval = function(id) {
    delete globalThis._timers[id];
    _updateActiveTimers();
};

globalThis.setTimeout = function(callback, delayMs) {
    const id = globalThis._timerId++;
    globalThis._timers[id] = {
        callback: callback,
        delay: (typeof delayMs === 'number' && delayMs >= 0) ? delayMs : 0,
        lastRun: Date.now(),
        once: true
    };
    _updateActiveTimers();
    return id;
};

globalThis.clearTimeout = function(id) {
    delete globalThis._timers[id];
    _updateActiveTimers();
};

globalThis._onTimerTick = function() {
    const now = Date.now();
    let hadTimeout = false;
    for (const id in globalThis._timers) {
        const timer = globalThis._timers[id];
        if (now - timer.lastRun >= timer.delay) {
            timer.lastRun = now;
            try { 
                timer.callback(); 
            } catch(e) {
                if (typeof host_error === "function") {
                    host_error("Timer callback error: " + (e && e.stack ? e.stack : e));
                }
            }
            if (timer.once) {
                delete globalThis._timers[id];
                hadTimeout = true;
            }
        }
    }
    if (hadTimeout) {
        _updateActiveTimers();
    }
};
