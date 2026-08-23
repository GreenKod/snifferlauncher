use super::decoder::get_app_icon_pixels_inner;
use crate::jni::bridge::vm;
use crossbeam_channel::{Receiver, Sender, unbounded};
use jni::errors::Error as JniError;
use jni::{Env, jni_sig, jni_str};
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};
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
static PENDING_REQUESTS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

fn pending_requests() -> &'static Mutex<HashSet<String>> {
    PENDING_REQUESTS.get_or_init(|| Mutex::new(HashSet::new()))
}

/// Initializes the 4-worker JNI async icon decoding channel pool.
pub fn init_icon_worker_pool() {
    if ICON_REQ_SENDER.get().is_some() {
        return;
    }

    let (req_tx, req_rx) = unbounded::<IconLoadRequest>();
    let (res_tx, res_rx) = unbounded::<IconLoadResult>();

    let _ = ICON_REQ_SENDER.set(req_tx);
    let _ = ICON_RES_RECEIVER.set(res_rx);

    for _ in 0..4 {
        let req_rx_clone = req_rx.clone();
        let res_tx_clone = res_tx.clone();

        thread::spawn(move || {
            let jvm = vm();
            let _ = jvm.attach_current_thread::<_, (), JniError>(|env: &mut Env| {
                if let Ok(looper_class) = env.find_class(jni_str!("android/os/Looper")) {
                    let _ = env.call_static_method(
                        looper_class,
                        jni_str!("prepare"),
                        jni_sig!("()V"),
                        &[],
                    );
                }

                while let Ok(req) = req_rx_clone.recv() {
                    let _ = env.with_local_frame(128, |env| {
                        if let Some((pixels, width, height)) =
                            get_app_icon_pixels_inner(env, &req.package_name)
                        {
                            let _ = res_tx_clone.send(IconLoadResult {
                                package_name: req.package_name.clone(),
                                pixels,
                                width,
                                height,
                            });
                        }
                        Ok::<(), JniError>(())
                    });
                    if let Ok(mut pending) = pending_requests().lock() {
                        pending.remove(&req.package_name);
                    }
                }
                Ok(())
            });
        });
    }
}

/// Request asynchronous loading of an application icon.
pub fn request_async_app_icon(package_name: &str) {
    if let Ok(mut pending) = pending_requests().lock() {
        if !pending.insert(package_name.to_string()) {
            return;
        }
    }

    if let Some(sender) = ICON_REQ_SENDER.get() {
        let _ = sender.send(IconLoadRequest {
            package_name: package_name.to_string(),
        });
    }
}

/// Enqueues a list of package names to the async JNI icon worker pool ahead of time.
pub fn prefetch_app_icons(package_names: &[String]) {
    for pkg in package_names {
        request_async_app_icon(pkg);
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
#[must_use]
pub fn get_app_icon_pixels(package_name: &str) -> Option<(Vec<u8>, u32, u32)> {
    let jvm = vm();

    jvm.attach_current_thread_for_scope::<_, _, JniError>(|env: &mut Env| {
        match get_app_icon_pixels_inner(env, package_name) {
            Some(res) => Ok(res),
            None => Err(JniError::JavaException),
        }
    })
    .ok()
}
