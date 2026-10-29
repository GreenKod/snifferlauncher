// =============================================================================
// SnifferUI — Generic Declarative UI Framework for SnifferEngine
// =============================================================================

const SnifferUI = (function () {
    let _renderFn = null;
    let _state = {};
    let _lastState = null;
    let _lastJson = null;
    let _eventHandlers = {};
    let _dismissHandlers = {};

    function _snapshotState(state) {
        if (!state || typeof state !== "object") return {};
        const snapshot = {};
        for (const key in state) {
            try {
                const val = state[key];
                if (typeof val !== "function") {
                    snapshot[key] = val;
                }
            } catch (e) {}
        }
        return snapshot;
    }

    function _shallowEqual(stateA, stateB) {
        if (stateA === stateB) return true;
        if (!stateA || !stateB || typeof stateA !== "object" || typeof stateB !== "object") {
            return false;
        }
        const keysA = Object.keys(stateA);
        const keysB = Object.keys(stateB);
        if (keysA.length !== keysB.length) return false;
        for (let i = 0; i < keysA.length; i++) {
            const key = keysA[i];
            if (!Object.prototype.hasOwnProperty.call(stateB, key) || stateA[key] !== stateB[key]) {
                return false;
            }
        }
        return true;
    }

    function _commit(force) {
        if (!_renderFn) return;

        const currentSnapshot = _snapshotState(_state);
        const isSameState = !force && _lastState !== null && _shallowEqual(currentSnapshot, _lastState);
        if (isSameState) {
            return;
        }

        const ui = _renderFn(_state);

        if (typeof host_set_ui_fast === "function") {
            _lastState = currentSnapshot;
            _lastJson = null;
            host_set_ui_fast(ui);
            return;
        }

        const json = JSON.stringify(ui);

        if (!force && json === _lastJson) {
            _lastState = currentSnapshot;
            return;
        }

        _lastState = currentSnapshot;
        _lastJson = json;
        host_set_ui(json);
    }

    function _matchId(idStr, targets) {
        if (!idStr) return false;
        const list = Array.isArray(targets) ? targets : [targets];
        for (let i = 0; i < list.length; i++) {
            const t = list[i];
            if (idStr === t) return true;
            if (typeof host_hash === "function" && idStr === String(host_hash(t))) return true;
        }
        return false;
    }

    return {
        start(renderFn, initialState) {
            _renderFn  = renderFn;
            _state     = initialState !== undefined && initialState !== null ? initialState : {};
            _lastState = null;
            _lastJson  = null;
            _commit(true);
        },

        getState() {
            return _state;
        },

        setState(patch) {
            if (patch && typeof patch === "object") {
                Object.assign(_state, patch);
            }
            _commit(false);
        },

        forceUpdate(force) {
            _commit(force === true);
        },

        matchId(idStr, targets) {
            return _matchId(idStr, targets);
        },

        on(eventType, targetOrHandler, maybeHandler) {
            if (!eventType) return;
            let handler = null;
            if (typeof targetOrHandler === "function") {
                handler = targetOrHandler;
            } else if (targetOrHandler && typeof maybeHandler === "function") {
                handler = function (e) {
                    if (_matchId(e && e.id, targetOrHandler)) {
                        return maybeHandler(e);
                    }
                    return false;
                };
            }
            if (!handler) return;
            if (!_eventHandlers[eventType]) {
                _eventHandlers[eventType] = [];
            }
            _eventHandlers[eventType].push(handler);
        },

        onBack(handler) {
            this.on("Back", handler);
        },

        onDismiss(actionOrHandler, maybeHandler) {
            if (typeof actionOrHandler === "function") {
                this.on("ScrollDismiss", actionOrHandler);
            } else if (typeof actionOrHandler === "string" && typeof maybeHandler === "function") {
                if (!_dismissHandlers[actionOrHandler]) {
                    _dismissHandlers[actionOrHandler] = [];
                }
                _dismissHandlers[actionOrHandler].push(maybeHandler);
            }
        },

        dispatchEvent(eventOrJson) {
            const e = typeof eventOrJson === "string" ? JSON.parse(eventOrJson) : eventOrJson;
            if (!e || !e.type) return false;

            if (e.type === "ScrollDismiss" && e.action && _dismissHandlers[e.action]) {
                const actionList = _dismissHandlers[e.action];
                for (let i = 0; i < actionList.length; i++) {
                    if (actionList[i](e) === true) return true;
                }
            }

            const handlers = _eventHandlers[e.type];
            if (handlers) {
                for (let i = 0; i < handlers.length; i++) {
                    if (handlers[i](e) === true) return true;
                }
            }

            if (e.type === "Back" && _state && typeof _state.onBack === "function") {
                if (_state.onBack(e) === true) return true;
            }

            if (e.type === "WindowResized") {
                _commit(true);
                return true;
            }

            return false;
        },
    };
})();

if (typeof globalThis.onEvent !== "function") {
    globalThis.onEvent = function (eventJsonString) {
        SnifferUI.dispatchEvent(eventJsonString);
        return "[]";
    };
}
