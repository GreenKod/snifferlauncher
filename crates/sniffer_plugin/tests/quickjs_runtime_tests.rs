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
