use rquickjs::{Context, Runtime};

#[test]
fn test_quickjs_basic_evaluation() {
    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();

    ctx.with(|c| {
        let val: i32 = c.eval("1 + 2 * 3").unwrap();
        assert_eq!(val, 7);

        let s: String = c.eval("`Hello ${'World'}`").unwrap();
        assert_eq!(s, "Hello World");
    });
}

#[test]
fn test_persistent_function() {
    use rquickjs::{Function, Persistent};
    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();
    let p_fn = ctx.with(|c| {
        c.eval::<(), _>("function foo() { return 42; }").unwrap();
        let f: Function = c.globals().get("foo").unwrap();
        Persistent::save(&c, f)
    });
    ctx.with(|c| {
        let f = p_fn.clone().restore(&c).unwrap();
        let val: i32 = f.call(()).unwrap();
        assert_eq!(val, 42);
    });
}

#[test]
fn test_quickjs_json_ast_generation() {
    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();

    let script = r#"
        function makeCard(id, label) {
            return {
                type: "Container",
                id: id,
                children: [
                    { type: "Label", text: label }
                ]
            };
        }
        JSON.stringify(makeCard("btn_1", "Chrome"));
    "#;

    ctx.with(|c| {
        let json_str: String = c.eval(script).unwrap();
        assert!(json_str.contains("btn_1"));
        assert!(json_str.contains("Chrome"));
    });
}

#[test]
fn test_timer_bindings_toggle_has_active_timers() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();
    let active_timers = Arc::new(AtomicBool::new(false));
    let active_timers_clone = Arc::clone(&active_timers);

    ctx.with(|c| {
        let globals = c.globals();
        globals
            .set(
                "host_set_active_timers",
                rquickjs::Function::new(c.clone(), move |active: bool| {
                    active_timers_clone.store(active, Ordering::SeqCst);
                })
                .unwrap(),
            )
            .unwrap();

        c.eval::<(), _>(sniffer_plugin::js::IPC_PREAMBLE).unwrap();

        assert!(!active_timers.load(Ordering::SeqCst));

        // 1. setInterval sets active_timers = true
        let int_id: u32 = c.eval("setInterval(() => {}, 1000)").unwrap();
        assert!(active_timers.load(Ordering::SeqCst));

        // 2. clearInterval sets active_timers = false
        c.eval::<(), _>(format!("clearInterval({int_id})")).unwrap();
        assert!(!active_timers.load(Ordering::SeqCst));

        // 3. setTimeout sets active_timers = true
        let _timeout_id: u32 = c.eval("setTimeout(() => {}, 0)").unwrap();
        assert!(active_timers.load(Ordering::SeqCst));

        // 4. _onTimerTick executes timeout and resets active_timers to false
        c.eval::<(), _>("_onTimerTick()").unwrap();
        assert!(!active_timers.load(Ordering::SeqCst));

        // 5. setTimeout and clearTimeout
        let t2: u32 = c.eval("setTimeout(() => {}, 5000)").unwrap();
        assert!(active_timers.load(Ordering::SeqCst));
        c.eval::<(), _>(format!("clearTimeout({t2})")).unwrap();
        assert!(!active_timers.load(Ordering::SeqCst));
    });
}

#[test]
fn test_persistent_on_timer_tick_caching() {
    use rquickjs::{Function, Persistent};
    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();

    let (persistent_tick, fired) = ctx.with(|c| {
        let fired = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let fired_clone = fired.clone();
        c.globals()
            .set(
                "host_set_active_timers",
                Function::new(c.clone(), |_active: bool| {}).unwrap(),
            )
            .unwrap();
        c.globals()
            .set(
                "mark_fired",
                Function::new(c.clone(), move || {
                    fired_clone.store(true, std::sync::atomic::Ordering::SeqCst);
                })
                .unwrap(),
            )
            .unwrap();

        c.eval::<(), _>(sniffer_plugin::js::IPC_PREAMBLE).unwrap();
        c.eval::<(), _>("setTimeout(() => { mark_fired(); }, 0);")
            .unwrap();

        let tick_fn: Function = c.globals().get("_onTimerTick").unwrap();
        (Persistent::save(&c, tick_fn), fired)
    });

    assert!(!fired.load(std::sync::atomic::Ordering::SeqCst));

    // Simulate worker thread tick loop: invoke persistent without lookup
    ctx.with(|c| {
        let handler = persistent_tick.clone().restore(&c).unwrap();
        handler.call::<_, ()>(()).unwrap();
    });

    assert!(fired.load(std::sync::atomic::Ordering::SeqCst));
}

#[test]
fn test_sniffer_ui_commit_shallow_dirty_check() {
    use rquickjs::Function;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();

    let commit_count = Arc::new(AtomicUsize::new(0));
    let commit_count_clone = commit_count.clone();

    let framework_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.plugins/framework/sniffer_ui.js");
    let framework_code =
        std::fs::read_to_string(&framework_path).expect("Failed to read sniffer_ui.js");

    ctx.with(|c| {
        // Register mock host_set_ui
        c.globals()
            .set(
                "host_set_ui",
                Function::new(c.clone(), move |_json: String| {
                    commit_count_clone.fetch_add(1, Ordering::SeqCst);
                })
                .unwrap(),
            )
            .unwrap();

        // Eval framework
        c.eval::<(), _>(framework_code.as_bytes()).unwrap();

        // 1. Initial start should trigger exactly 1 commit
        let test_script = r#"
            let state = { count: 0, text: "hello" };
            function render() {
                return { type: "Label", text: state.text + " " + state.count };
            }
            SnifferUI.start(render, state);
        "#;
        c.eval::<(), _>(test_script).unwrap();
        assert_eq!(
            commit_count.load(Ordering::SeqCst),
            1,
            "Initial start must commit UI"
        );

        // 2. forceUpdate() with identical state should NOT call host_set_ui
        c.eval::<(), _>("SnifferUI.forceUpdate();").unwrap();
        assert_eq!(
            commit_count.load(Ordering::SeqCst),
            1,
            "Redundant forceUpdate must skip commit"
        );

        // 3. setState with identical values should NOT call host_set_ui
        c.eval::<(), _>("SnifferUI.setState({ count: 0 });")
            .unwrap();
        assert_eq!(
            commit_count.load(Ordering::SeqCst),
            1,
            "setState with same value must skip commit"
        );

        // 4. setState with changed values SHOULD trigger commit
        c.eval::<(), _>("SnifferUI.setState({ count: 1 });")
            .unwrap();
        assert_eq!(
            commit_count.load(Ordering::SeqCst),
            2,
            "setState with new value must commit"
        );

        // 5. In-place state property mutation followed by forceUpdate() SHOULD trigger commit
        c.eval::<(), _>("state.count = 2; SnifferUI.forceUpdate();")
            .unwrap();
        assert_eq!(
            commit_count.load(Ordering::SeqCst),
            3,
            "In-place mutation followed by forceUpdate must commit"
        );

        // 6. Another forceUpdate() without mutating anything should skip
        c.eval::<(), _>("SnifferUI.forceUpdate();").unwrap();
        assert_eq!(
            commit_count.load(Ordering::SeqCst),
            3,
            "Second forceUpdate without mutation must skip"
        );

        // 7. Explicit forceUpdate(true) should bypass state check and force commit
        c.eval::<(), _>("SnifferUI.forceUpdate(true);").unwrap();
        assert_eq!(
            commit_count.load(Ordering::SeqCst),
            4,
            "forceUpdate(true) must bypass dirty check"
        );
    });
}
