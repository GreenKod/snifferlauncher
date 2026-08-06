use super::{context, vm};
use crossbeam_channel::{Receiver, Sender, unbounded};
use jni::errors::Error as JniError;
use jni::objects::JValue;
use jni::{Env, jni_sig, jni_str};
use std::sync::OnceLock;
use std::thread;

pub struct IconLoadRequest {
    pub package_name: String,
}

pub struct IconLoadResult {
    pub package_name: String,
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

static ICON_REQ_SENDER: OnceLock<Sender<IconLoadRequest>> = OnceLock::new();
static ICON_RES_RECEIVER: OnceLock<Receiver<IconLoadResult>> = OnceLock::new();

/// Initializes the 2-worker JNI async icon decoding channel pool.
pub fn init_icon_worker_pool() {
    if ICON_REQ_SENDER.get().is_some() {
        return;
    }

    let (req_tx, req_rx) = unbounded::<IconLoadRequest>();
    let (res_tx, res_rx) = unbounded::<IconLoadResult>();

    let _ = ICON_REQ_SENDER.set(req_tx);
    let _ = ICON_RES_RECEIVER.set(res_rx);

    for _ in 0..2 {
        let req_rx_clone = req_rx.clone();
        let res_tx_clone = res_tx.clone();

        thread::spawn(move || {
            while let Ok(req) = req_rx_clone.recv() {
                if let Some((pixels, width, height)) = get_app_icon_pixels(&req.package_name) {
                    let _ = res_tx_clone.send(IconLoadResult {
                        package_name: req.package_name,
                        pixels,
                        width,
                        height,
                    });
                }
            }
        });
    }
}

/// Request asynchronous loading of an application icon.
pub fn request_async_app_icon(package_name: &str) {
    if let Some(sender) = ICON_REQ_SENDER.get() {
        let _ = sender.send(IconLoadRequest {
            package_name: package_name.to_string(),
        });
    }
}

/// Try to receive completed async icon decode results (non-blocking).
pub fn poll_async_app_icon() -> Option<IconLoadResult> {
    if let Some(receiver) = ICON_RES_RECEIVER.get() {
        receiver.try_recv().ok()
    } else {
        None
    }
}

/// Retrieves the application icon for a given package name and returns it as a raw RGBA pixel buffer.
pub(crate) fn extract_drawable_pixels(
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
