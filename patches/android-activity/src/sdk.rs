jni::bind_java_type! { pub(crate) IBinder => "android.os.IBinder" }
jni::bind_java_type! {
    pub(crate) View => "android.view.View",
    type_map {
        IBinder => "android.os.IBinder",
    },
    methods {
        fn get_window_token() -> IBinder,
    }
}
jni::bind_java_type! {
    pub(crate) InputMethodManager => "android.view.inputmethod.InputMethodManager",
    type_map {
        View => "android.view.View",
        IBinder => "android.os.IBinder",
    },
    methods {
        fn show_soft_input(view: View, flags: i32) -> bool,
        fn hide_soft_input_from_window(window_token: IBinder, flags: i32) -> bool,
    }
}
jni::bind_java_type! {
    pub(crate) Context => "android.content.Context",
    fields {
        #[allow(non_snake_case)]
        static INPUT_METHOD_SERVICE: JString
    },
    methods {
        fn get_system_service(service_name: JString) -> JObject,
    }
}
jni::bind_java_type! {
    pub(crate) Window => "android.view.Window",
    type_map {
        View => "android.view.View",
    },
    methods {
        fn get_decor_view() -> View,
    }
}
jni::bind_java_type! {
    pub(crate) Activity => "android.app.Activity",
    type_map {
        Context => "android.content.Context",
        Window => "android.view.Window",
    },
    is_instance_of {
        context: Context
    },
    methods {
        fn get_window() -> Window,
    }
}

// Explicitly initialize the JNI bindings so we can get and early, upfront,
// error if something is wrong.
pub(crate) fn jni_init(_env: &jni::Env) -> jni::errors::Result<()> {
    Ok(())
}
