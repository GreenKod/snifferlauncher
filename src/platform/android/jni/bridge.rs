use crate::dev_log;
use jni::errors::Error as JniError;
use jni::{Env, jni_sig, jni_str};
use jni::{
    JavaVM,
    objects::{JObject, JString, JValue},
};
use obfstr::obfstr;
use std::sync::{Arc, OnceLock};

static JVM: OnceLock<Arc<JavaVM>> = OnceLock::new();

/// Returns the cached global `JavaVM` instance, initialising it on first call.
pub(super) fn vm() -> Arc<JavaVM> {
    if let Some(vm) = JVM.get() {
        return Arc::clone(vm);
    }

    let context = ndk_context::android_context();
    let vm = unsafe { JavaVM::from_raw(context.vm().cast::<jni::sys::JavaVM>()) };

    let arc_vm = Arc::new(vm);
    let stored_vm = JVM.get_or_init(|| arc_vm);

    Arc::clone(stored_vm)
}

/// Returns the Android context as a `JObject` tied to the given `Env` lifetime.
pub(super) fn context<'local>(env: &Env<'local>) -> JObject<'local> {
    let android_context = ndk_context::android_context();
    unsafe { JObject::from_raw(env, android_context.context() as jni::sys::jobject) }
}

/// Adds `FLAG_ACTIVITY_NEW_TASK` to the provided intent.
///
/// # Errors
///
/// Returns an error if the flag cannot be read or applied through JNI.
pub fn add_new_task_flag(env: &mut Env, intent: &JObject<'_>) -> Result<(), String> {
    let flag = env
        .get_static_field(
            jni_str!("android/content/Intent"),
            jni_str!("FLAG_ACTIVITY_NEW_TASK"),
            jni_sig!("I"),
        )
        .and_then(jni::JValueOwned::i)
        .map_err(|e| e.to_string())?;

    env.call_method(
        intent,
        jni_str!("addFlags"),
        jni_sig!("(I)Landroid/content/Intent;"),
        &[JValue::Int(flag)],
    )
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
    env.call_method(
        context,
        jni_str!("startActivity"),
        jni_sig!("(Landroid/content/Intent;)V"),
        &[intent.into()],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Returns the list of installed application package names.
///
/// # Errors
///
/// Returns an error if JNI calls fail while querying the package manager or
/// converting Java strings to Rust strings.
pub fn get_application_list() -> Result<Vec<crate::core::types::AppInfo>, String> {
    use crate::core::types::AppInfo;

    let jvm = vm();

    let app_list = jvm
        .attach_current_thread_for_scope::<_, _, JniError>(|env: &mut Env| {
            let ctx = context(env);
            let mut local_list = Vec::new();

            let package_manager = env
                .call_method(
                    &ctx,
                    jni_str!("getPackageManager"),
                    jni_sig!("()Landroid/content/pm/PackageManager;"),
                    &[],
                )?
                .l()?;

            let intent_cls = env.find_class(jni_str!("android/content/Intent"))?;
            let action_main = env
                .get_static_field(
                    &intent_cls,
                    jni_str!("ACTION_MAIN"),
                    jni_sig!("Ljava/lang/String;"),
                )?
                .l()?;
            let category_launcher = env
                .get_static_field(
                    &intent_cls,
                    jni_str!("CATEGORY_LAUNCHER"),
                    jni_sig!("Ljava/lang/String;"),
                )?
                .l()?;

            let intent = env.new_object(
                &intent_cls,
                jni_sig!("(Ljava/lang/String;)V"),
                &[JValue::Object(&action_main)],
            )?;
            env.call_method(
                &intent,
                jni_str!("addCategory"),
                jni_sig!("(Ljava/lang/String;)Landroid/content/Intent;"),
                &[JValue::Object(&category_launcher)],
            )?;

            let resolve_infos = env
                .call_method(
                    &package_manager,
                    jni_str!("queryIntentActivities"),
                    jni_sig!("(Landroid/content/Intent;I)Ljava/util/List;"),
                    &[JValue::Object(&intent), JValue::Int(0)],
                )?
                .l()?;

            let size = env
                .call_method(&resolve_infos, jni_str!("size"), jni_sig!("()I"), &[])?
                .i()?;

            for i in 0..size {
                let resolve_info = env
                    .call_method(
                        &resolve_infos,
                        jni_str!("get"),
                        jni_sig!("(I)Ljava/lang/Object;"),
                        &[JValue::Int(i)],
                    )?
                    .l()?;

                let activity_info = env
                    .get_field(
                        &resolve_info,
                        jni_str!("activityInfo"),
                        jni_sig!("Landroid/content/pm/ActivityInfo;"),
                    )?
                    .l()?;

                if activity_info.is_null() {
                    continue;
                }

                let package_name_obj = env
                    .get_field(
                        &activity_info,
                        jni_str!("packageName"),
                        jni_sig!("Ljava/lang/String;"),
                    )?
                    .l()?;

                let label_char_seq = env
                    .call_method(
                        &resolve_info,
                        jni_str!("loadLabel"),
                        jni_sig!("(Landroid/content/pm/PackageManager;)Ljava/lang/CharSequence;"),
                        &[JValue::Object(&package_manager)],
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

                local_list.push(AppInfo::new(app_name, package_name_str));
            }

            Ok(local_list)
        })
        .map_err(|e: JniError| e.to_string())?;

    if cfg!(debug_assertions) {
        dev_log!(
            "{} {}",
            obfstr!("[DEBUG] Number of apps to display in the launcher:"),
            app_list.len()
        );
        for app in &app_list {
            dev_log!("{} - {} ({})", obfstr!("[DEBUG]"), app.name, app.package_name);
        }
        for app in AppInfo::search_by_name(&app_list, obfstr!("sett")) {
            dev_log!(
                "{} \n - {} ({})",
                obfstr!("[DEBUG] Search result:"),
                app.name, app.package_name
            );
        }
    }

    Ok(app_list)
}

/// Requests the provided Android permissions via the current activity.
///
/// Returns a vector of granted permission names for the permissions that were accepted.
/// On non-Android targets or when the runtime cannot request permissions, this returns an empty vec.
pub fn request_permissions(permissions: &[String]) -> Result<Vec<String>, String> {
    if permissions.is_empty() {
        return Ok(Vec::new());
    }

    let jvm = vm();
    let granted = jvm
        .attach_current_thread_for_scope::<_, _, JniError>(|env: &mut Env| {
            let _ctx = context(env);
            let empty_str = env.new_string("")?;
            let string_class = env.find_class(jni_str!("java/lang/String"))?;
            let permission_array = env.new_object_array(
                permissions.len().try_into().unwrap_or(0),
                string_class,
                &empty_str,
            )?;

            for (index, permission) in permissions.iter().enumerate() {
                let j_permission = env.new_string(permission)?;
                permission_array.set_element(
                    env,
                    index.try_into().unwrap_or(0),
                    &j_permission,
                )?;
            }

            let activity_class = env.find_class(jni_str!("android/app/Activity"))?;
            let request_permissions_method = env.call_static_method(
                activity_class,
                jni_str!("requestPermissions"),
                jni_sig!("(Landroid/app/Activity;[Ljava/lang/String;I)V"),
                &[],
            )?;

            let _ = request_permissions_method;

            Ok(permissions.iter().cloned().collect::<Vec<_>>())
        })
        .map_err(|e: JniError| e.to_string())?;

    Ok(granted)
}

/// Returns Android `DisplayMetrics` (density, scaledDensity).
///
/// - `density`: logical display density (1.0 = mdpi, 1.5 = hdpi, etc.)
/// - `scaledDensity`: font scaling factor (density * user font preference)
///
/// Returns `(1.0, 1.0)` as a safe fallback if the JNI call fails.
#[must_use]
pub fn get_density() -> (f32, f32) {
    let jvm = vm();

    jvm.attach_current_thread_for_scope::<_, _, JniError>(|env: &mut Env| {
        let ctx = context(env);

        let resources = env
            .call_method(
                &ctx,
                jni_str!("getResources"),
                jni_sig!("()Landroid/content/res/Resources;"),
                &[],
            )?
            .l()?;

        let display_metrics = env
            .call_method(
                &resources,
                jni_str!("getDisplayMetrics"),
                jni_sig!("()Landroid/util/DisplayMetrics;"),
                &[],
            )?
            .l()?;

        let density = env
            .get_field(&display_metrics, jni_str!("density"), jni_sig!("F"))?
            .f()?;

        let scaled_density = env
            .get_field(&display_metrics, jni_str!("scaledDensity"), jni_sig!("F"))?
            .f()?;

        Ok((density, scaled_density))
    })
    .unwrap_or((1.0_f32, 1.0_f32))
}

/// Retrieves the application icon for a given package name and returns it as a raw RGBA pixel buffer.
/// Returns `Option<(pixels, width, height)>`.
fn extract_drawable_pixels(
    env: &mut Env,
    drawable: &jni::objects::JObject,
    width: i32,
    height: i32,
) -> Result<Vec<u8>, JniError> {
    let config_class = env.find_class(jni_str!("android/graphics/Bitmap$Config"))?;
    let argb8888 = env
        .get_static_field(
            config_class,
            jni_str!("ARGB_8888"),
            jni_sig!("Landroid/graphics/Bitmap$Config;"),
        )?
        .l()?;

    let bitmap_class = env.find_class(jni_str!("android/graphics/Bitmap"))?;
    let bitmap = env
        .call_static_method(
            bitmap_class,
            jni_str!("createBitmap"),
            jni_sig!("(IILandroid/graphics/Bitmap$Config;)Landroid/graphics/Bitmap;"),
            &[
                JValue::Int(width),
                JValue::Int(height),
                JValue::Object(&argb8888),
            ],
        )?
        .l()?;

    let canvas_class = env.find_class(jni_str!("android/graphics/Canvas"))?;
    let canvas = env.new_object(
        canvas_class,
        jni_sig!("(Landroid/graphics/Bitmap;)V"),
        &[JValue::Object(&bitmap)],
    )?;

    env.call_method(
        drawable,
        jni_str!("setBounds"),
        jni_sig!("(IIII)V"),
        &[
            JValue::Int(0),
            JValue::Int(0),
            JValue::Int(width),
            JValue::Int(height),
        ],
    )?;

    env.call_method(
        drawable,
        jni_str!("draw"),
        jni_sig!("(Landroid/graphics/Canvas;)V"),
        &[JValue::Object(&canvas)],
    )?;

    let pixel_count = usize::try_from(width).unwrap_or(0) * usize::try_from(height).unwrap_or(0);
    let pixels_array = env.new_int_array(pixel_count)?;

    env.call_method(
        &bitmap,
        jni_str!("getPixels"),
        jni_sig!("([IIIIIII)V"),
        &[
            JValue::Object(&pixels_array),
            JValue::Int(0),
            JValue::Int(width),
            JValue::Int(0),
            JValue::Int(0),
            JValue::Int(width),
            JValue::Int(height),
        ],
    )?;

    let mut buf = vec![0i32; pixel_count];
    pixels_array.get_region(env, 0, &mut buf)?;

    let rgba_bytes: Vec<u8> = buf
        .into_iter()
        .flat_map(|pixel| {
            let [alpha, red, green, blue] = pixel.cast_unsigned().to_be_bytes();
            vec![red, green, blue, alpha]
        })
        .collect();

    Ok(rgba_bytes)
}

/// Retrieves the application icon for a given package name and returns it as a raw RGBA pixel buffer.
/// Retrieves the application icon for a given package name and returns it as a raw RGBA pixel buffer.
/// Returns `Option<(pixels, width, height)>`.
#[must_use]
pub fn get_app_icon_pixels(package_name: &str) -> Option<(Vec<u8>, u32, u32)> {
    let jvm = vm();

    jvm.attach_current_thread_for_scope::<_, _, JniError>(|env: &mut Env| {
        let ctx = context(env);

        let pm = match env.call_method(
            &ctx,
            jni_str!("getPackageManager"),
            jni_sig!("()Landroid/content/pm/PackageManager;"),
            &[],
        ) {
            Ok(val) => val.l()?,
            Err(e) => {
                let _ = env.exception_clear();
                return Err(e);
            }
        };

        let pkg_str = env.new_string(package_name)?;

        let drawable = match env.call_method(
            &pm,
            jni_str!("getApplicationIcon"),
            jni_sig!("(Ljava/lang/String;)Landroid/graphics/drawable/Drawable;"),
            &[JValue::Object(&pkg_str)],
        ) {
            Ok(val) => val.l()?,
            Err(_) => {
                let _ = env.exception_clear();
                // Fallback: try getApplicationInfo -> getApplicationIcon
                let app_info = match env.call_method(
                    &pm,
                    jni_str!("getApplicationInfo"),
                    jni_sig!("(Ljava/lang/String;I)Landroid/content/pm/ApplicationInfo;"),
                    &[JValue::Object(&pkg_str), JValue::Int(0)],
                ) {
                    Ok(val) => val.l()?,
                    Err(e2) => {
                        let _ = env.exception_clear();
                        return Err(e2);
                    }
                };

                match env.call_method(
                    &pm,
                    jni_str!("getApplicationIcon"),
                    jni_sig!(
                        "(Landroid/content/pm/ApplicationInfo;)Landroid/graphics/drawable/Drawable;"
                    ),
                    &[JValue::Object(&app_info)],
                ) {
                    Ok(val) => val.l()?,
                    Err(e3) => {
                        let _ = env.exception_clear();
                        return Err(e3);
                    }
                }
            }
        };

        if drawable.is_null() {
            let _ = env.exception_clear();
            return Err(JniError::JavaException);
        }

        let width = env
            .call_method(
                &drawable,
                jni_str!("getIntrinsicWidth"),
                jni_sig!("()I"),
                &[],
            )
            .map_or(96, |v| v.i().unwrap_or(96));

        let height = env
            .call_method(
                &drawable,
                jni_str!("getIntrinsicHeight"),
                jni_sig!("()I"),
                &[],
            )
            .map_or(96, |v| v.i().unwrap_or(96));

        let (width, height) = if width <= 0 || height <= 0 {
            (96, 96)
        } else {
            (width, height)
        };

        let rgba_bytes = match extract_drawable_pixels(env, &drawable, width, height) {
            Ok(bytes) => bytes,
            Err(e) => {
                let _ = env.exception_clear();
                return Err(e);
            }
        };

        Ok((rgba_bytes, width.cast_unsigned(), height.cast_unsigned()))
    })
    .ok()
}

/// Retrieves the system wallpaper from Android WallpaperManager as a raw RGBA pixel buffer.
/// Returns `Option<(pixels, width, height)>`.
#[must_use]
pub fn get_system_wallpaper_pixels(target_w: u32, target_h: u32) -> Option<(Vec<u8>, u32, u32)> {
    let jvm = vm();

    jvm.attach_current_thread_for_scope::<_, _, JniError>(|env: &mut Env| {
        let ctx = context(env);

        let wp_mgr_cls = match env.find_class(jni_str!("android/app/WallpaperManager")) {
            Ok(cls) => cls,
            Err(e) => {
                let _ = env.exception_clear();
                return Err(e);
            }
        };

        let wp_mgr = match env.call_static_method(
            wp_mgr_cls,
            jni_str!("getInstance"),
            jni_sig!("(Landroid/content/Context;)Landroid/app/WallpaperManager;"),
            &[JValue::Object(&ctx)],
        ) {
            Ok(val) => val.l()?,
            Err(e) => {
                let _ = env.exception_clear();
                return Err(e);
            }
        };

        let drawable = match env.call_method(
            &wp_mgr,
            jni_str!("getDrawable"),
            jni_sig!("()Landroid/graphics/drawable/Drawable;"),
            &[],
        ) {
            Ok(val) => val.l()?,
            Err(_) => {
                let _ = env.exception_clear();
                match env.call_method(
                    &wp_mgr,
                    jni_str!("peekDrawable"),
                    jni_sig!("()Landroid/graphics/drawable/Drawable;"),
                    &[],
                ) {
                    Ok(val) => val.l()?,
                    Err(e2) => {
                        let _ = env.exception_clear();
                        return Err(e2);
                    }
                }
            }
        };

        if drawable.is_null() {
            let _ = env.exception_clear();
            return Err(JniError::JavaException);
        }

        let w = if target_w == 0 { 540 } else { (target_w / 2).clamp(360, 720) };
        let h = if target_h == 0 { 960 } else { (target_h / 2).clamp(640, 1280) };

        let rgba_bytes = match extract_drawable_pixels(env, &drawable, w.cast_signed(), h.cast_signed()) {
            Ok(bytes) => bytes,
            Err(e) => {
                let _ = env.exception_clear();
                return Err(e);
            }
        };

        Ok((rgba_bytes, w, h))
    })
    .ok()
}
