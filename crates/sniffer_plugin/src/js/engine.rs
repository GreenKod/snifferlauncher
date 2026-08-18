use rquickjs::{Context, Runtime};

pub fn create_engine() -> Result<(Runtime, Context), String> {
    let runtime = Runtime::new().map_err(|e| format!("QuickJS runtime error: {e}"))?;
    let context = Context::full(&runtime).map_err(|e| format!("QuickJS context error: {e}"))?;
    Ok((runtime, context))
}

