// =============================================================================
// SnifferUI — React-like mini framework for SnifferLauncher plugins
// =============================================================================

const SnifferUI = (function () {
    let _renderFn = null;
    let _state = {};

    function _commit() {
        if (_renderFn) {
            host_set_ui(JSON.stringify(_renderFn()));
        }
    }

    return {
        start(renderFn, initialState) {
            _renderFn = renderFn;
            _state    = initialState !== undefined && initialState !== null ? initialState : {};
            _commit();
        },

        setState(patch) {
            Object.assign(_state, patch);
            _commit();
        },

        forceUpdate() {
            _commit();
        },
    };
})();
