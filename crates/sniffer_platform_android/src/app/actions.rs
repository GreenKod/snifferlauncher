use android_activity::AndroidApp;
use jni::errors::Error as JniError;
use jni::objects::JValue;
use jni::{Env, jni_sig, jni_str};
use obfstr::obfstr;
use std::sync::{Arc, Mutex};

pub fn process_android_actions(
    action_queue: &Arc<Mutex<Vec<sniffer_core::types::Action>>>,
    app: &AndroidApp,
) {
    if let Ok(mut q) = action_queue.lock() {
        let mut remaining = Vec::new();
        for action in q.drain(..) {
            match action {
                sniffer_core::Action::FocusTextInput(_id) => {
                    let jvm = crate::jni::bridge::vm();
                    let _ =
                        jvm.attach_current_thread_for_scope::<_, _, JniError>(|env: &mut Env| {
                            let ctx = crate::jni::bridge::context(env);
                            if !ctx.is_null() {
                                if let Ok(win_val) = env.call_method(
                                    &ctx,
                                    jni_str!("getWindow"),
                                    jni_sig!("()Landroid/view/Window;"),
                                    &[],
                                ) {
                                    if let Ok(win) = win_val.l() {
                                        if !win.is_null() {
                                            // 1. Android 11+ (API 30+): WindowInsetsController
                                            if let Ok(ctrl_val) = env.call_method(
                                                &win,
                                                jni_str!("getInsetsController"),
                                                jni_sig!("()Landroid/view/WindowInsetsController;"),
                                                &[],
                                            ) {
                                                if let Ok(ctrl) = ctrl_val.l() {
                                                    if !ctrl.is_null() {
                                                        let _ = env.call_method(
                                                            &ctrl,
                                                            jni_str!("show"),
                                                            jni_sig!("(I)V"),
                                                            &[JValue::Int(8)], // WindowInsets.Type.ime()
                                                        );
                                                        let _ = env.exception_clear();
                                                    }
                                                }
                                            }
                                            let _ = env.exception_clear();

                                            // 2. Request focus on DecorView
                                            if let Ok(dv_val) = env.call_method(
                                                &win,
                                                jni_str!("getDecorView"),
                                                jni_sig!("()Landroid/view/View;"),
                                                &[],
                                            ) {
                                                if let Ok(dv) = dv_val.l() {
                                                    if !dv.is_null() {
                                                        let _ = env.call_method(
                                                            &dv,
                                                            jni_str!("setFocusableInTouchMode"),
                                                            jni_sig!("(Z)V"),
                                                            &[JValue::Bool(true)],
                                                        );
                                                        let _ = env.exception_clear();
                                                        let _ = env.call_method(
                                                            &dv,
                                                            jni_str!("requestFocus"),
                                                            jni_sig!("()Z"),
                                                            &[],
                                                        );
                                                        let _ = env.exception_clear();
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                // 3. Fallback / direct InputMethodManager toggle
                                if let Ok(imm_str) = env.new_string("input_method") {
                                    if let Ok(imm_val) = env.call_method(
                                        &ctx,
                                        jni_str!("getSystemService"),
                                        jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"),
                                        &[(&imm_str).into()],
                                    ) {
                                        if let Ok(imm) = imm_val.l() {
                                            if !imm.is_null() {
                                                let _ = env.call_method(
                                                    &imm,
                                                    jni_str!("toggleSoftInput"),
                                                    jni_sig!("(II)V"),
                                                    &[JValue::Int(2), JValue::Int(0)], // SHOW_FORCED
                                                );
                                                let _ = env.exception_clear();
                                            }
                                        }
                                    }
                                }
                                let _ = env.exception_clear();
                            }
                            Ok(())
                        });
                    app.show_soft_input(false);
                }
                sniffer_core::Action::BlurTextInput => {
                    let jvm = crate::jni::bridge::vm();
                    let _ = jvm.attach_current_thread_for_scope::<_, _, JniError>(|env: &mut Env| {
                        let ctx = crate::jni::bridge::context(env);
                        if !ctx.is_null() {
                            if let Ok(win_val) = env.call_method(
                                &ctx,
                                jni_str!("getWindow"),
                                jni_sig!("()Landroid/view/Window;"),
                                &[],
                            ) {
                                if let Ok(win) = win_val.l() {
                                    if !win.is_null() {
                                        if let Ok(ctrl_val) = env.call_method(
                                            &win,
                                            jni_str!("getInsetsController"),
                                            jni_sig!("()Landroid/view/WindowInsetsController;"),
                                            &[],
                                        ) {
                                            if let Ok(ctrl) = ctrl_val.l() {
                                                if !ctrl.is_null() {
                                                    let _ = env.call_method(
                                                        &ctrl,
                                                        jni_str!("hide"),
                                                        jni_sig!("(I)V"),
                                                        &[JValue::Int(8)], // WindowInsets.Type.ime()
                                                    );
                                                    let _ = env.exception_clear();
                                                }
                                            }
                                        }
                                        let _ = env.exception_clear();

                                        if let Ok(dv_val) = env.call_method(
                                            &win,
                                            jni_str!("getDecorView"),
                                            jni_sig!("()Landroid/view/View;"),
                                            &[],
                                        ) {
                                            if let Ok(dv) = dv_val.l() {
                                                if !dv.is_null() {
                                                    if let Ok(token_val) = env.call_method(
                                                        &dv,
                                                        jni_str!("getWindowToken"),
                                                        jni_sig!("()Landroid/os/IBinder;"),
                                                        &[],
                                                    ) {
                                                        if let Ok(token) = token_val.l() {
                                                            if !token.is_null() {
                                                                if let Ok(imm_str) = env.new_string("input_method") {
                                                                    if let Ok(imm_val) = env.call_method(
                                                                        &ctx,
                                                                        jni_str!("getSystemService"),
                                                                        jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"),
                                                                        &[(&imm_str).into()],
                                                                    ) {
                                                                        if let Ok(imm) = imm_val.l() {
                                                                            if !imm.is_null() {
                                                                                let _ = env.call_method(
                                                                                    &imm,
                                                                                    jni_str!("hideSoftInputFromWindow"),
                                                                                    jni_sig!("(Landroid/os/IBinder;I)Z"),
                                                                                    &[(&token).into(), JValue::Int(0)],
                                                                                );
                                                                                let _ = env.exception_clear();
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            let _ = env.exception_clear();
                        }
                        Ok(())
                    });
                    app.hide_soft_input(false);
                }
                sniffer_core::Action::LoadImage { id, src } => {
                    remaining.push(sniffer_core::Action::LoadImage { id, src });
                }
                _ => {
                    if let Err(e) = crate::jni::intent::launch_action(action) {
                        sniffer_core::dev_err!("{}: {e}", obfstr!("Failed to launch action"));
                    }
                }
            }
        }
        *q = remaining;
    }
}
