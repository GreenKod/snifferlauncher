use super::{context, vm};
use jni::errors::Error as JniError;
use jni::objects::JValue;
use jni::{Env, jni_sig, jni_str};

/// Opens the Android system settings dialog for setting the Default Home/Launcher app.
pub fn open_default_home_picker() {
    let jvm = vm();
    let _ = jvm.attach_current_thread_for_scope::<_, _, JniError>(|env: &mut Env| {
        let ctx = context(env);
        let intent_cls = env.find_class(jni_str!("android/content/Intent"))?;

        let action_str = env.new_string("android.settings.HOME_SETTINGS")?;
        let intent = env.new_object(
            &intent_cls,
            jni_sig!("(Ljava/lang/String;)V"),
            &[JValue::Object(&action_str)],
        )?;

        let flag = env
            .get_static_field(
                &intent_cls,
                jni_str!("FLAG_ACTIVITY_NEW_TASK"),
                jni_sig!("I"),
            )?
            .i()?;

        let _ = env.call_method(
            &intent,
            jni_str!("addFlags"),
            jni_sig!("(I)Landroid/content/Intent;"),
            &[JValue::Int(flag)],
        );

        let _ = env.call_method(
            &ctx,
            jni_str!("startActivity"),
            jni_sig!("(Landroid/content/Intent;)V"),
            &[JValue::Object(&intent)],
        );

        Ok(())
    });
}
