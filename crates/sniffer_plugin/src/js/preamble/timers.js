// =============================================================================
// Timers Polyfill (setInterval, setTimeout, clearInterval, clearTimeout)
// =============================================================================

globalThis._timers = {};
globalThis._timerId = 1;

globalThis.setInterval = function(callback, delayMs) {
    const id = globalThis._timerId++;
    globalThis._timers[id] = {
        callback: callback,
        delay: (typeof delayMs === 'number' && delayMs >= 0) ? delayMs : 1000,
        lastRun: Date.now(),
        once: false
    };
    return id;
};

globalThis.clearInterval = function(id) {
    delete globalThis._timers[id];
};

globalThis.setTimeout = function(callback, delayMs) {
    const id = globalThis._timerId++;
    globalThis._timers[id] = {
        callback: callback,
        delay: (typeof delayMs === 'number' && delayMs >= 0) ? delayMs : 0,
        lastRun: Date.now(),
        once: true
    };
    return id;
};

globalThis.clearTimeout = function(id) {
    delete globalThis._timers[id];
};

globalThis._onTimerTick = function() {
    const now = Date.now();
    for (const id in globalThis._timers) {
        const timer = globalThis._timers[id];
        if (now - timer.lastRun >= timer.delay) {
            timer.lastRun = now;
            try { timer.callback(); } catch(e) {}
            if (timer.once) {
                delete globalThis._timers[id];
            }
        }
    }
};
