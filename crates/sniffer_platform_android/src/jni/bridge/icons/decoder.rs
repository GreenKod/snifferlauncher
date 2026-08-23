use super::super::context;
use jni::errors::Error as JniError;
use jni::objects::JValue;
use jni::{Env, jni_sig, jni_str};

pub(crate) fn extract_drawable_pixels(
    env: &mut Env,
    drawable: &jni::objects::JObject,
    width: i32,
    height: i32,
) -> Result<Vec<u8>, JniError> {
    let _ = env.call_method(
        drawable,
        jni_str!("mutate"),
        jni_sig!("()Landroid/graphics/drawable/Drawable;"),
        &[],
    );
    env.exception_clear();

    let config_class = env.find_class(jni_str!("android/graphics/Bitmap$Config"))?;
    let argb8888 = env
        .get_static_field(
            config_class,
            jni_str!("ARGB_8888"),
            jni_sig!("Landroid/graphics/Bitmap$Config;"),
        )?
        .l()?;

    let ctx = context(env);
    let display_metrics = env
        .call_method(
            &ctx,
            jni_str!("getResources"),
            jni_sig!("()Landroid/content/res/Resources;"),
            &[],
        )
        .and_then(|res| {
            let res_obj = res.l()?;
            env.call_method(
                &res_obj,
                jni_str!("getDisplayMetrics"),
                jni_sig!("()Landroid/util/DisplayMetrics;"),
                &[],
            )?
            .l()
        });

    let bitmap_class = env.find_class(jni_str!("android/graphics/Bitmap"))?;

    let bitmap = if let Ok(dm) = display_metrics {
        env.call_static_method(
            bitmap_class,
            jni_str!("createBitmap"),
            jni_sig!("(Landroid/util/DisplayMetrics;IILandroid/graphics/Bitmap$Config;)Landroid/graphics/Bitmap;"),
            &[
                JValue::Object(&dm),
                JValue::Int(width),
                JValue::Int(height),
                JValue::Object(&argb8888),
            ],
        )?
        .l()?
    } else {
        env.exception_clear();
        env.call_static_method(
            bitmap_class,
            jni_str!("createBitmap"),
            jni_sig!("(IILandroid/graphics/Bitmap$Config;)Landroid/graphics/Bitmap;"),
            &[
                JValue::Int(width),
                JValue::Int(height),
                JValue::Object(&argb8888),
            ],
        )?
        .l()?
    };

    let canvas_class = env.find_class(jni_str!("android/graphics/Canvas"))?;
    let canvas = env.new_object(
        canvas_class,
        jni_sig!("(Landroid/graphics/Bitmap;)V"),
        &[JValue::Object(&bitmap)],
    )?;

    let _ = env.call_method(
        &canvas,
        jni_str!("drawColor"),
        jni_sig!("(I)V"),
        &[JValue::Int(0)],
    );
    env.exception_clear();

    let _ = env.call_method(
        drawable,
        jni_str!("setAlpha"),
        jni_sig!("(I)V"),
        &[JValue::Int(255)],
    );
    let _ = env.call_method(
        drawable,
        jni_str!("setVisible"),
        jni_sig!("(ZZ)Z"),
        &[JValue::Bool(true), JValue::Bool(false)],
    );
    env.exception_clear();

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

pub(crate) fn get_icon_cache_dir_path(env: &mut Env) -> std::path::PathBuf {
    let ctx = context(env);
    let path_res: Result<String, JniError> = (|| {
        let files_dir = env
            .call_method(
                &ctx,
                jni_str!("getCacheDir"),
                jni_sig!("()Ljava/io/File;"),
                &[],
            )?
            .l()?;
        let path_obj = env
            .call_method(
                &files_dir,
                jni_str!("getAbsolutePath"),
                jni_sig!("()Ljava/lang/String;"),
                &[],
            )?
            .l()?;
        let path_jstring = env.as_cast::<jni::objects::JString>(&path_obj)?;
        let path_str = path_jstring.try_to_string(env)?;
        Ok(path_str)
    })();

    let base =
        path_res.unwrap_or_else(|_| "/data/data/com.greenkod.snifferlauncher/cache".to_string());
    let dir = std::path::PathBuf::from(base).join("icons");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub(crate) fn load_icon_from_disk(
    env: &mut Env,
    package_name: &str,
) -> Option<(Vec<u8>, u32, u32)> {
    let dir = get_icon_cache_dir_path(env);
    let path = dir.join(format!("{package_name}.raw"));
    if let Ok(data) = std::fs::read(&path) {
        if data.len() >= 8 {
            let width = u32::from_le_bytes(data[0..4].try_into().ok()?);
            let height = u32::from_le_bytes(data[4..8].try_into().ok()?);
            let expected = 8 + (width as usize) * (height as usize) * 4;
            if data.len() == expected {
                let pixels = data[8..].to_vec();
                return Some((pixels, width, height));
            }
        }
        let _ = std::fs::remove_file(path);
    }
    None
}

pub(crate) fn save_icon_to_disk(
    env: &mut Env,
    package_name: &str,
    pixels: &[u8],
    width: u32,
    height: u32,
) {
    let dir = get_icon_cache_dir_path(env);
    let path = dir.join(format!("{package_name}.raw"));
    let tmp_path = dir.join(format!("{package_name}.tmp"));
    let mut data = Vec::with_capacity(8 + pixels.len());
    data.extend_from_slice(&width.to_le_bytes());
    data.extend_from_slice(&height.to_le_bytes());
    data.extend_from_slice(pixels);
    if std::fs::write(&tmp_path, data).is_ok() {
        let _ = std::fs::rename(&tmp_path, path);
    }
}

pub(crate) fn get_app_icon_pixels_inner(
    env: &mut Env,
    package_name: &str,
) -> Option<(Vec<u8>, u32, u32)> {
    if let Some(cached) = load_icon_from_disk(env, package_name) {
        return Some(cached);
    }

    let result: Result<(Vec<u8>, u32, u32), JniError> = (|| {
        let ctx = context(env);

        let pm = match env.call_method(
            &ctx,
            jni_str!("getPackageManager"),
            jni_sig!("()Landroid/content/pm/PackageManager;"),
            &[],
        ) {
            Ok(val) => val.l()?,
            Err(e) => {
                env.exception_clear();
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
                env.exception_clear();
                let app_info = match env.call_method(
                    &pm,
                    jni_str!("getApplicationInfo"),
                    jni_sig!("(Ljava/lang/String;I)Landroid/content/pm/ApplicationInfo;"),
                    &[JValue::Object(&pkg_str), JValue::Int(128)],
                ) {
                    Ok(val) => val.l()?,
                    Err(e2) => {
                        env.exception_clear();
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
                        env.exception_clear();
                        return Err(e3);
                    }
                }
            }
        };

        if drawable.is_null() {
            env.exception_clear();
            return Err(JniError::JavaException);
        }

        let (width, height) = (96, 96);

        let rgba_bytes = match extract_drawable_pixels(env, &drawable, width, height) {
            Ok(bytes) => bytes,
            Err(e) => {
                env.exception_clear();
                return Err(e);
            }
        };

        let w = width.cast_unsigned();
        let h = height.cast_unsigned();
        save_icon_to_disk(env, package_name, &rgba_bytes, w, h);

        Ok((rgba_bytes, w, h))
    })();

    result.ok()
}
