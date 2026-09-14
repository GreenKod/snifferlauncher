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
