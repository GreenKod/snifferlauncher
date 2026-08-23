use android_activity::AndroidApp;
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
                    app.show_soft_input(true);
                }
                sniffer_core::Action::BlurTextInput => {
                    app.hide_soft_input(true);
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
