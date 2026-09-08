use super::bridge::{RenderMessage, SharedRenderState, collect_layout_rects};
use crate::window::EglContextState;
use android_activity::AndroidApp;
use obfstr::obfstr;
use sniffer_core::{Action, Renderer, dev_err, dev_log};
use sniffer_pkg::PackageRegistry;
use sniffer_render::draw::draw_ui;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

#[allow(clippy::too_many_lines)]
pub fn spawn_render_thread(
    app: AndroidApp,
    shared_render_state: Arc<RwLock<Arc<SharedRenderState>>>,
    action_queue: Arc<Mutex<Vec<Action>>>,
    profiler: Arc<Mutex<sniffer_core::profiler::FrameProfiler>>,
    pkg_registry: Arc<RwLock<PackageRegistry>>,
    render_rx: crossbeam_channel::Receiver<RenderMessage>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut egl_state: Option<EglContextState> = None;
        let mut is_window_bound = false;

        loop {
            while let Ok(msg) = render_rx.try_recv() {
                match msg {
                    RenderMessage::InitWindow(ptr, _w, _h) => {
                        if egl_state.is_none() {
                            match EglContextState::new() {
                                Ok(s) => egl_state = Some(s),
                                Err(e) => dev_err!(
                                    "{}: {e}",
                                    obfstr!("[EGL Error] Failed to create EGL state")
                                ),
                            }
                        }
                        if let Some(ref mut egl) = egl_state {
                            if let Err(e) = egl.bind_window(ptr as *mut std::ffi::c_void) {
                                dev_err!("{}: {e}", obfstr!("[EGL Error] Failed to bind window"));
                            } else {
                                dev_log!("{}", obfstr!("[EGL] Window successfully bound to EGL"));
                                is_window_bound = true;
                            }
                        }
                    }
                    RenderMessage::WindowResized(_ptr, _w, _h) => {
                        is_window_bound = true;
                    }
                    RenderMessage::TerminateWindow => {
                        if let Some(ref mut egl) = egl_state {
                            egl.unbind();
                            is_window_bound = false;
                        }
                    }
                    RenderMessage::LowMemory => {
                        if let Some(ref mut egl) = egl_state {
                            if let Some(ref mut renderer) = egl.renderer {
                                renderer.trim_memory();
                            }
                        }
                        if let Ok(reg) = pkg_registry.read() {
                            reg.trim_memory(sniffer_pkg::MemoryTrimLevel::Critical);
                        }
                    }
                    RenderMessage::Destroy => {
                        if let Ok(mut reg) = pkg_registry.write() {
                            reg.unload_all();
                        }
                        return;
                    }
                }
            }

            if is_window_bound {
                if let Some(ref mut egl) = egl_state {
                    if let Some(ref mut renderer) = egl.renderer {
                        if let Ok(mut q) = action_queue.lock() {
                            let mut unhandled = Vec::new();
                            let mut loaded_textures = 0;

                            for action in q.drain(..) {
                                if loaded_textures >= 16 {
                                    unhandled.push(action);
                                    continue;
                                }
                                match action {
                                    Action::LoadImage { id: _, src } => {
                                        if let Some(pkg_name) =
                                            src.strip_prefix(obfstr!("app-icon://"))
                                        {
                                            crate::jni::bridge::request_async_app_icon(pkg_name);
                                        } else {
                                            let asset_path =
                                                format!("{}/{}", obfstr!(".plugins"), src);
                                            if let Ok(cstr) =
                                                std::ffi::CString::new(asset_path.clone())
                                            {
                                                if let Some(mut asset) =
                                                    app.asset_manager().open(cstr.as_c_str())
                                                {
                                                    use std::io::Read;
                                                    let mut buffer = Vec::new();
                                                    if asset.read_to_end(&mut buffer).is_ok() {
                                                        if let Ok(img) =
                                                            image::load_from_memory(&buffer)
                                                        {
                                                            let rgba = img.to_rgba8();
                                                            let (w, h) = rgba.dimensions();
                                                            renderer.load_image(
                                                                &src,
                                                                rgba.as_raw(),
                                                                w,
                                                                h,
                                                            );
                                                            loaded_textures += 1;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    _ => {
                                        unhandled.push(action);
                                    }
                                }
                            }

                            if !unhandled.is_empty() {
                                *q = unhandled;
                            }
                        }

                        let render_start = std::time::Instant::now();
                        let current_state = shared_render_state.read().unwrap().clone();

                        let width = current_state.metrics.physical_width;
                        let height = current_state.metrics.physical_height;

                        while let Some(res) = crate::jni::bridge::poll_async_app_icon() {
                            let image_id =
                                format!("{}{}", obfstr!("app-icon://"), res.package_name);
                            renderer.load_image(&image_id, &res.pixels, res.width, res.height);
                        }

                        renderer.begin_frame(width, height);

                        if !current_state.shaders_warmed_up {
                            renderer.warm_up_shaders();
                            if let Ok(mut rs) = shared_render_state.write() {
                                let mut updated = (**rs).clone();
                                updated.shaders_warmed_up = true;
                                *rs = Arc::new(updated);
                            }
                        }

                        renderer.clear(0x0000_0000);

                        #[allow(unused_variables)]
                        let rendered_nodes = draw_ui(
                            renderer,
                            &current_state.root_element,
                            &current_state.layout_tree,
                            &current_state.metrics,
                            &current_state.style_map,
                            &current_state.data_map,
                            &current_state.transition_manager,
                            1.0,
                            0.0,
                            0.0,
                        );

                        let mut layout_rects = std::collections::HashMap::new();
                        collect_layout_rects(&current_state.layout_tree, &mut layout_rects);
                        if let Ok(mut reg) = pkg_registry.write() {
                            reg.render_widgets(renderer, &layout_rects, None);
                        }

                        #[cfg(feature = "devkit")]
                        {
                            if let Ok(prof) = profiler.lock() {
                                sniffer_core::profiler::render_devkit_debug_overlays(
                                    renderer,
                                    &prof,
                                    &current_state.root_element,
                                    &current_state.layout_tree,
                                );
                            }
                        }

                        renderer.end_frame();
                        let draw_end = std::time::Instant::now();

                        egl.swap_buffers();
                        let swap_end = std::time::Instant::now();

                        if let Ok(mut prof) = profiler.lock() {
                            let total_nodes =
                                sniffer_core::profiler::count_elements(&current_state.root_element);
                            prof.record_entities(rendered_nodes, total_nodes);
                            prof.record_frame(render_start, render_start, draw_end, swap_end);
                        }
                    }
                }
            } else {
                std::thread::sleep(Duration::from_millis(16));
            }
        }
    })
}
