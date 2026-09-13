use super::{context, vm};
use jni::errors::Error as JniError;
use jni::{Env, jni_sig, jni_str};

/// Requests the provided Android permissions via the current activity.
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
                permission_array.set_element(env, index, &j_permission)?;
            }

            let activity_class = env.find_class(jni_str!("android/app/Activity"))?;
            let _res = env.call_static_method(
                activity_class,
                jni_str!("requestPermissions"),
                jni_sig!("(Landroid/app/Activity;[Ljava/lang/String;I)V"),
                &[],
            )?;

            Ok(permissions.to_vec())
        })
        .map_err(|e: JniError| e.to_string())?;

    Ok(granted)
}
