use crate::core::component::Action;
use android_activity::AndroidApp;
use jni::{Env, jni_sig, jni_str};
use jni::{
    JavaVM,
    objects::{JObject, JValue},
};
use std::sync::{Arc, OnceLock};
static JVM: OnceLock<Arc<JavaVM>> = OnceLock::new();

/// Launches a specific Android system or application action.
///
/// # Errors
///
/// Returns an error if the Android JNI calls fail or if the requested action
/// cannot be started by the system.
pub fn launch_action(action: Action) -> Result<(), String> {
    match action {
        Action::OpenSettings => start_action("android.settings.SETTINGS"),
        Action::OpenContacts => start_view_uri("content://contacts/people"),
        Action::OpenCamera => start_action("android.media.action.STILL_IMAGE_CAMERA"),
    }
}

#[must_use]
pub fn get_safe_area(app: &AndroidApp) -> Option<(i32, i32)> {
    // 1. get GlobalJvm
    let jvm = vm();

    // 2. Explicitly declare the JNI error return type to the compiler
    jvm.attach_current_thread_for_scope::<_, _, jni::errors::Error>(|env: &mut Env| {
        // 3. Check for null pointer
        let activity_ptr = app.activity_as_ptr() as jni::sys::jobject;
        if activity_ptr.is_null() {
            return Ok(None);
        }

        // 4. Convert to JObject
        let activity = unsafe { jni::objects::JObject::from_raw(env, activity_ptr) };
        if activity.is_null() {
            return Ok(None);
        }

        // 5. Call Java method chain
        let window = env
            .call_method(
                &activity,
                jni_str!("getWindow"),
                jni_sig!("()Landroid/view/Window;"),
                &[],
            )?
            .l()?;

        let decor_view = env
            .call_method(
                &window,
                jni_str!("getDecorView"),
                jni_sig!("()Landroid/view/View;"),
                &[],
            )?
            .l()?;

        let insets = env
            .call_method(
                &decor_view,
                jni_str!("getRootWindowInsets"),
                jni_sig!("()Landroid/view/WindowInsets;"),
                &[],
            )?
            .l()?;

        if insets.is_null() {
            return Ok(None);
        }

        // 6. Find the Top and Bottom values
        let top = env
            .call_method(
                &insets,
                jni_str!("getSystemWindowInsetTop"),
                jni_sig!("()I"),
                &[],
            )?
            .i()?;

        let bottom = env
            .call_method(
                &insets,
                jni_str!("getSystemWindowInsetBottom"),
                jni_sig!("()I"),
                &[],
            )?
            .i()?;

        //  Export the result
        Ok(Some((top, bottom)))
    })
    .ok() // Converts Result to Option
    .flatten() // Clears unnecessary Option
}

fn start_action(action: &str) -> Result<(), String> {
    // 1. get GlobalJvm
    let jvm = vm();

    // 2. Explicitly declare the JNI error return type to the compiler
    jvm.attach_current_thread_for_scope::<_, _, jni::errors::Error>(|env: &mut Env| {
        let context = context(env);

        // 3. Convert the Rust string to a Java String object and start a new Intent
        let action_string = env.new_string(action)?;
        let intent = env.new_object(
            jni_str!("android/content/Intent"),
            jni_sig!("(Ljava/lang/String;)V"),
            &[(&action_string).into()],
        )?;

        // 4. Wrap the String errors returned from our own functions with jni::errors::Error::JavaException
        add_new_task_flag(env, &intent).map_err(|_| jni::errors::Error::JavaException)?;

        start_activity(env, &context, &intent).map_err(|_| jni::errors::Error::JavaException)?;

        Ok(())
    })
    // 5. Convert the JNI error that goes out of closure to String, which is the actual return type of the function
    .map_err(|e| e.to_string())
}

/// Opens the provided URI with the Android VIEW intent.
///
/// # Errors
///
/// Returns an error if URI parsing, intent construction, or activity launch
/// fails.
pub fn start_view_uri(uri: &str) -> Result<(), String> {
    // 1. get GlobalJvm
    let jvm = vm();

    // 2. Explicitly declare the JNI error return type to the compiler
    jvm.attach_current_thread_for_scope::<_, _, jni::errors::Error>(|env: &mut Env| {
        // 3. Get the Android context to be used in the startActivity call
        let context = context(env);

        // 4. We prepare the parameters to be used in Java methods (Uri.parse and new Intent)
        let action_string = env.new_string("android.intent.action.VIEW")?;
        let uri_string = env.new_string(uri)?;

        // 5. Java equivalent: Uri parsedUri = Uri.parse(uriString);
        let parsed_uri = env
            .call_static_method(
                jni_str!("android/net/Uri"),
                jni_str!("parse"),
                jni_sig!("(Ljava/lang/String;)Landroid/net/Uri;"),
                &[(&uri_string).into()],
            )?
            .l()?;

        // 6. Create a new Intent with two arguments by passing the action and URI objects we have prepared by parameterizing
        let intent = env.new_object(
            jni_str!("android/content/Intent"),
            jni_sig!("(Ljava/lang/String;Landroid/net/Uri;)V"),
            &[(&action_string).into(), (&parsed_uri).into()],
        )?;

        // 7. Wrap the String errors returned from our own functions with jni::errors::Error::JavaException
        add_new_task_flag(env, &intent).map_err(|_| jni::errors::Error::JavaException)?;

        start_activity(env, &context, &intent).map_err(|_| jni::errors::Error::JavaException)?;

        Ok(())
    })
    // 7. Convert the JNI error that goes out of closure to String, which is the actual return type of the function
    .map_err(|e| e.to_string())
}

/// Adds `FLAG_ACTIVITY_NEW_TASK` to the provided intent.
///
/// # Errors
///
/// Returns an error if the flag cannot be read or applied through JNI.
pub fn add_new_task_flag(env: &mut Env, intent: &JObject<'_>) -> Result<(), String> {
    // 1. Fetch Android's static "NEW_TASK" flag value to open a new screen from the background
    let flag = env
        .get_static_field(
            jni_str!("android/content/Intent"),
            jni_str!("FLAG_ACTIVITY_NEW_TASK"),
            jni_sig!("I"),
        )
        .and_then(jni::JValueOwned::i)
        .map_err(|e| e.to_string())?;

    // 2. Invoke the addFlags method on the Intent object to apply the flag
    env.call_method(
        intent,
        jni_str!("addFlags"),
        jni_sig!("(I)Landroid/content/Intent;"),
        &[JValue::Int(flag)],
    )
    // 3. Wrap any JNI errors that may occur during this process into a String error
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Starts an Android activity using the provided context and intent.
///
/// # Errors
///
/// Returns an error if `startActivity` fails through JNI.
pub fn start_activity(
    env: &mut Env,
    context: &JObject<'_>,
    intent: &JObject<'_>,
) -> Result<(), String> {
    // 1. Invoke the startActivity method on the context (which is the Activity) to launch the Intent
    env.call_method(
        context,
        jni_str!("startActivity"),
        jni_sig!("(Landroid/content/Intent;)V"),
        &[intent.into()],
    )
    // 2. Wrap any JNI errors that may occur during this process into a String error
    .map_err(|e| e.to_string())?;

    Ok(())
}

fn vm() -> Arc<JavaVM> {
    // 1. Check if the JVM is already stored in the global variable, if it is, we return it directly
    if let Some(vm) = JVM.get() {
        return Arc::clone(vm);
    }

    // 2. If the JVM is not stored, we create it from the Android context and store it in the global variable
    let context = ndk_context::android_context();

    // 3. Create a JavaVM instance from the raw pointer obtained from the Android context and wrap it in an Arc for thread safety
    let vm = unsafe { JavaVM::from_raw(context.vm().cast::<jni::sys::JavaVM>()) };

    let arc_vm = Arc::new(vm);
    let stored_vm = JVM.get_or_init(|| arc_vm);

    Arc::clone(stored_vm)
}

fn context<'local>(env: &Env<'local>) -> JObject<'local> {
    // 1. Get the Android context using the ndk_context crate, which provides a safe wrapper around the raw Android context pointer
    let android_context = ndk_context::android_context();

    // 2. Convert the raw Android context pointer to a JObject that can be used in JNI calls. We use the from_raw method to create a JObject from the raw pointer, and we ensure that the lifetime of the JObject is tied to the Env reference to prevent dangling references.
    unsafe { JObject::from_raw(env, android_context.context() as jni::sys::jobject) }
}
