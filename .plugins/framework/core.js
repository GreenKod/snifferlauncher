// =============================================================================
// SnifferUI — React-like mini framework for SnifferLauncher plugins
// =============================================================================

const SnifferUI = (function () {
    let _renderFn = null;
    let _state = {};
    let _lastState = null;
    let _lastJson = null;

    function _snapshotState(state) {
        if (!state || typeof state !== "object") return {};
        const snapshot = {};
        const keys = Object.keys(state);
        for (let i = 0; i < keys.length; i++) {
            const key = keys[i];
            const desc = Object.getOwnPropertyDescriptor(state, key);
            if (desc && !desc.get && typeof desc.value !== "function") {
                snapshot[key] = desc.value;
            }
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
    };
})();
