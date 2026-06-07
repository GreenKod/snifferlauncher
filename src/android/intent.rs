use crate::core::component::Action;
use android_activity::AndroidApp;
use jni::errors::Error as JniError;
use jni::{Env, jni_sig, jni_str};
use jni::{
    JavaVM,
    objects::{JObject, JString, JValue},
};
use std::sync::{Arc, OnceLock};
static JVM: OnceLock<Arc<JavaVM>> = OnceLock::new();

#[derive(Debug)]
pub struct AppInfo {
    pub name: String,
    pub package_name: String,
    pub icon_bytes: Vec<u8>,
}

impl AppInfo {
    #[must_use]
    pub const fn new(name: String, package_name: String, icon_bytes: Vec<u8>) -> Self {
        Self {
            name,
            package_name,
            icon_bytes,
        }
    }

    /// Creates an `AppInfo` instance with only basic information and no icon.
    /// Icon data can be added later using `with_icon`.
    #[must_use]
    pub const fn from_package_info(name: String, package_name: String) -> Self {
        Self {
            name,
            package_name,
            icon_bytes: Vec::new(),
        }
    }

    /// Adds or updates the app icon bytes using a builder-style pattern.
    #[must_use]
    pub fn with_icon(mut self, icon_bytes: Vec<u8>) -> Self {
        self.icon_bytes = icon_bytes;
        self
    }

    /// Filters apps by name for the launcher search bar.
    /// `Q: AsRef<str>` allows both `&str` and `String` to be passed directly.
    pub fn search_by_name<Q: AsRef<str>>(apps: &[Self], query: Q) -> Vec<&Self> {
        let query_str = query.as_ref().to_lowercase();
        if query_str.is_empty() {
            return apps.iter().collect(); // Return all apps when the query is empty
        }

        apps.iter()
            .filter(|app| app.name.to_lowercase().contains(&query_str))
            .collect()
    }
}

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

/// Returns the list of installed application package names.
///
/// # Errors
///
/// Returns an error if JNI calls fail while querying the package manager or
/// converting Java strings to Rust strings.
pub fn get_aplication_list() -> Result<Vec<AppInfo>, String> {
    let jvm = vm();

    let app_list = jvm
        .attach_current_thread_for_scope::<_, _, JniError>(|env: &mut Env| {
            let context = context(env);
            let mut local_list = Vec::new();

            let package_manager = env
                .call_method(
                    &context,
                    jni_str!("getPackageManager"),
                    jni_sig!("()Landroid/content/pm/PackageManager;"),
                    &[],
                )?
                .l()?;

            // Use the GET_ACTIVITIES (512) flag to include clickable activities
            let packages = env
                .call_method(
                    &package_manager,
                    jni_str!("getInstalledPackages"),
                    jni_sig!("(I)Ljava/util/List;"),
                    &[JValue::Int(512)],
                )?
                .l()?;

            let size = env
                .call_method(&packages, jni_str!("size"), jni_sig!("()I"), &[])?
                .i()?;

            for i in 0..size {
                let package_info = env
                    .call_method(
                        &packages,
                        jni_str!("get"),
                        jni_sig!("(I)Ljava/lang/Object;"),
                        &[JValue::Int(i)],
                    )?
                    .l()?;

                let package_name_obj = env
                    .get_field(
                        &package_info,
                        jni_str!("packageName"),
                        jni_sig!("Ljava/lang/String;"),
                    )?
                    .l()?;

                // Launcher check: filter out hidden services without a home screen icon
                let launch_intent = env
                    .call_method(
                        &package_manager,
                        jni_str!("getLaunchIntentForPackage"),
                        jni_sig!("(Ljava/lang/String;)Landroid/content/Intent;"),
                        &[JValue::Object(&package_name_obj)],
                    )?
                    .l()?;

                if launch_intent.is_null() {
                    continue;
                }

                let app_info_obj = env
                    .get_field(
                        &package_info,
                        jni_str!("applicationInfo"),
                        jni_sig!("Landroid/content/pm/ApplicationInfo;"),
                    )?
                    .l()?;

                let label_char_seq = env
                    .call_method(
                        &package_manager,
                        jni_str!("getApplicationLabel"),
                        jni_sig!("(Landroid/content/pm/ApplicationInfo;)Ljava/lang/CharSequence;"),
                        &[JValue::Object(&app_info_obj)],
                    )?
                    .l()?;

                let label_jstring = env
                    .call_method(
                        &label_char_seq,
                        jni_str!("toString"),
                        jni_sig!("()Ljava/lang/String;"),
                        &[],
                    )?
                    .l()?;

                let label_jstring = env.as_cast::<JString>(&label_jstring)?;
                let app_name = label_jstring.try_to_string(env)?;

                let package_name_jstring = env.as_cast::<JString>(&package_name_obj)?;
                let package_name_str = package_name_jstring.try_to_string(env)?;

                let app_info = AppInfo::new(app_name, package_name_str, Vec::new());

                local_list.push(app_info);
            }

            Ok(local_list)
        })
        .map_err(|e: JniError| e.to_string())?;

    if cfg!(debug_assertions) {
        println!(
            "[DEBUG] Number of apps to display in the launcher: {}",
            app_list.len()
        );
        for app in &app_list {
            println!("[DEBUG] - {} ({})", app.name, app.package_name);
        }
        for app in AppInfo::search_by_name(&app_list, "sett") {
            println!(
                "[DEBUG] Search result: \n - {} ({})",
                app.name, app.package_name
            );
        }
    }

    Ok(app_list)
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
