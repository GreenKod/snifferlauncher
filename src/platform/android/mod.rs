#[cfg(target_os = "android")]
pub mod jni;
#[cfg(target_os = "android")]
pub mod types;

#[cfg(target_os = "android")]
pub use jni::intent::launch_action;

#[cfg(not(target_os = "android"))]
/// Launch the requested action on non-Android targets.
///
/// # Errors
///
/// Always returns an error because action launching is only implemented on Android.
pub fn launch_action(_action: crate::core::Action) -> Result<(), String> {
    Err("android launcher is only available on Android".to_string())
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
#[allow(clippy::pedantic)]
pub fn android_main(app: android_activity::AndroidApp) {
    use crate::core::render::draw::{draw_ui, find_clicked_button, find_hovered_button};
    use crate::core::style::BACKGROUND;
    use crate::core::ui::data_map::DataMap;
    use crate::core::ui::event::{EventBus, UiEvent};
    use crate::core::ui::style_map::StyleMap;
    use crate::core::{
        Application, GlowRenderer, LauncherApp, LauncherMessage, LauncherState, Point, Renderer,
        ScreenMetrics, Size, calculate_layout,
    };
    use crate::plugin::registry::PluginRegistry;
    use android_activity::{
        InputStatus, MainEvent, PollEvent, input::InputEvent, input::MotionAction,
    };
    use std::time::Duration;

    // Helper struct to manage EGL resources dynamically across window creation/destruction
    struct EglContextState {
        egl: khronos_egl::DynamicInstance,
        display: khronos_egl::Display,
        config: khronos_egl::Config,
        context: khronos_egl::Context,
        surface: Option<khronos_egl::Surface>,
        renderer: Option<GlowRenderer>,
    }

    impl EglContextState {
        fn new() -> Result<Self, String> {
            let egl =
                unsafe { khronos_egl::DynamicInstance::<khronos_egl::EGL1_5>::load_required() }
                    .map_err(|e| format!("Failed to load EGL library: {}", e))?;

            let display = unsafe {
                egl.get_display(khronos_egl::DEFAULT_DISPLAY)
                    .ok_or_else(|| "Failed to get EGL display".to_string())?
            };

            egl.initialize(display)
                .map_err(|e| format!("Failed to initialize EGL: {:?}", e))?;

            let attribs = [
                khronos_egl::SURFACE_TYPE,
                khronos_egl::WINDOW_BIT,
                khronos_egl::RENDERABLE_TYPE,
                khronos_egl::OPENGL_ES3_BIT,
                khronos_egl::BLUE_SIZE,
                8,
                khronos_egl::GREEN_SIZE,
                8,
                khronos_egl::RED_SIZE,
                8,
                khronos_egl::ALPHA_SIZE,
                8,
                khronos_egl::NONE,
            ];

            let config = egl
                .choose_first_config(display, &attribs)
                .map_err(|e| format!("Failed to choose EGL config: {:?}", e))?
                .ok_or_else(|| "No EGL config found".to_string())?;

            let context_attribs = [
                khronos_egl::CONTEXT_CLIENT_VERSION,
                3, // Request ES 3.0 context
                khronos_egl::NONE,
            ];

            let context = egl
                .create_context(display, config, None, &context_attribs)
                .map_err(|e| format!("Failed to create EGL context: {:?}", e))?;

            Ok(Self {
                egl,
                display,
                config,
                context,
                surface: None,
                renderer: None,
            })
        }

        fn bind_window(&mut self, window: &ndk::native_window::NativeWindow) -> Result<(), String> {
            self.unbind();

            let native_window_ptr = window.ptr().as_ptr().cast::<std::ffi::c_void>();
            let surface = unsafe {
                self.egl
                    .create_window_surface(self.display, self.config, native_window_ptr, None)
                    .map_err(|e| format!("Failed to create EGL window surface: {:?}", e))?
            };

            self.egl
                .make_current(
                    self.display,
                    Some(surface),
                    Some(surface),
                    Some(self.context),
                )
                .map_err(|e| format!("Failed to make EGL context current: {:?}", e))?;

            self.surface = Some(surface);

            if self.renderer.is_none() {
                let gl = unsafe {
                    glow::Context::from_loader_function(|name| {
                        self.egl
                            .get_proc_address(name)
                            .map_or(std::ptr::null(), |f| f as *const _)
                    })
                };
                // Audiowide font gömülü olarak binary'ye dahil edildi
                static FONT_BYTES: &[u8] = include_bytes!("../../fonts/audiowide.ttf");
                let glow_renderer = unsafe { GlowRenderer::with_font(gl, Some(FONT_BYTES))? };
                self.renderer = Some(glow_renderer);
            }

            Ok(())
        }

        fn unbind(&mut self) {
            if let Some(surface) = self.surface.take() {
                let _ = self.egl.make_current(self.display, None, None, None);
                let _ = self.egl.destroy_surface(self.display, surface);
            }
        }

        fn swap_buffers(&self) {
            if let Some(surface) = self.surface {
                let _ = self.egl.swap_buffers(self.display, surface);
            }
        }
    }

    impl Drop for EglContextState {
        fn drop(&mut self) {
            self.unbind();
            let _ = self.egl.destroy_context(self.display, self.context);
            let _ = self.egl.terminate(self.display);
        }
    }

    let mut egl_state: Option<EglContextState> = None;
    let mut state = LauncherState::default();
    let mut running = true;
    let mut last_touch_pos = Point::zero();

    // Initialize the event-driven plugin system
    let event_bus = EventBus::default();
    let style_map = StyleMap::default();
    let data_map = DataMap::default();

    let mut plugin_registry = PluginRegistry::default();

    // In Android, we'd normally extract this to internal storage, but for now we read from local .plugins
    let loader = crate::plugin::PluginLoader::new(".plugins");
    loader.register_all(&mut plugin_registry);

    while running {
        app.poll_events(Some(Duration::from_millis(16)), |event| match event {
            PollEvent::Wake | PollEvent::Timeout => {
                if let Some(ref mut egl) = egl_state
                    && let Some(ref mut renderer) = egl.renderer
                    && let Some(window) = app.native_window()
                {
                    let width =
                        f32::from(u16::try_from(window.width()).expect("window width fits in u16"));
                    let height = f32::from(
                        u16::try_from(window.height()).expect("window height fits in u16"),
                    );

                    let (safe_area_top, safe_area_bottom) =
                        crate::platform::android::jni::get_safe_area(&app)
                            .map(|(top, bottom)| {
                                (
                                    f32::from(
                                        i16::try_from(top).expect("safe-area top fits in i16"),
                                    ),
                                    f32::from(
                                        i16::try_from(bottom)
                                            .expect("safe-area bottom fits in i16"),
                                    ),
                                )
                            })
                            .unwrap_or((0.0, 0.0));

                    // Build responsive screen metrics from Android DisplayMetrics
                    let (density, scaled_density) = crate::platform::android::jni::get_density();
                    let metrics = ScreenMetrics::from_scale(
                        width,
                        height - safe_area_top - safe_area_bottom,
                        density,
                        scaled_density,
                    );

                    let root_element = LauncherApp::view(&state, &metrics);
                    let layout_tree = calculate_layout(
                        &root_element,
                        Size::new(width, height - safe_area_top - safe_area_bottom),
                        0.0,
                        safe_area_top,
                    );

                    // Dispatch any queued events to plugins BEFORE rendering
                    plugin_registry.dispatch(&event_bus, &style_map, &data_map);

                    renderer.begin_frame(width, height);
                    renderer.clear(BACKGROUND);

                    draw_ui(renderer, &root_element, &layout_tree, &metrics, &style_map, &data_map);

                    renderer.end_frame();

                    egl.swap_buffers();
                }
            }
            PollEvent::Main(main_event) => match main_event {
                MainEvent::InitWindow { .. } => {
                    if egl_state.is_none() {
                        egl_state = EglContextState::new().ok();
                    }
                    if let Some(ref mut egl) = egl_state
                        && let Some(window) = app.native_window()
                    {
                        let _ = egl.bind_window(&window);
                    }
                }
                MainEvent::WindowResized { .. }
                | MainEvent::ContentRectChanged { .. }
                | MainEvent::RedrawNeeded { .. } => {
                    if let Some(ref mut egl) = egl_state
                        && let Some(window) = app.native_window()
                    {
                        let _ = egl.bind_window(&window);
                    }
                }
                MainEvent::InputAvailable => {
                    if let Ok(mut iter) = app.input_events_iter() {
                        loop {
                            let had_event = iter.next(|input_event| match input_event {
                                InputEvent::MotionEvent(motion_event) => {
                                    let pointer =
                                        motion_event.pointer_at_index(motion_event.pointer_index());
                                    let point = Point::new(pointer.raw_x(), pointer.raw_y());
                                    last_touch_pos = point;

                                    app.native_window()
                                        .map_or(InputStatus::Unhandled, |window| {
                                            let width = f32::from(
                                                u16::try_from(window.width())
                                                    .expect("window width fits in u16"),
                                            );
                                            let height = f32::from(
                                                u16::try_from(window.height())
                                                    .expect("window height fits in u16"),
                                            );
                                            let (safe_area_top, safe_area_bottom) =
                                                crate::platform::android::jni::get_safe_area(&app)
                                                    .map(|(top, bottom)| {
                                                        (
                                                            f32::from(i16::try_from(top).expect(
                                                                "safe-area top fits in i16",
                                                            )),
                                                            f32::from(
                                                                i16::try_from(bottom).expect(
                                                                    "safe-area bottom fits in i16",
                                                                ),
                                                            ),
                                                        )
                                                    })
                                                    .unwrap_or((0.0, 0.0));

                                            // Build metrics for touch-path view, same as render path
                                            let (density, scaled_density) =
                                                crate::platform::android::jni::get_density();
                                            let metrics = ScreenMetrics::from_scale(
                                                width,
                                                height - safe_area_top - safe_area_bottom,
                                                density,
                                                scaled_density,
                                            );

                                            let root_element = LauncherApp::view(&state, &metrics);
                                            let layout_tree = calculate_layout(
                                                &root_element,
                                                Size::new(
                                                    width,
                                                    height - safe_area_top - safe_area_bottom,
                                                ),
                                                0.0,
                                                safe_area_top,
                                            );

                                            match motion_event.action() {
                                                MotionAction::Down
                                                | MotionAction::Move
                                                | MotionAction::PointerDown => {
                                                    let prev_hovered = state.hovered;
                                                    let hovered_btn = find_hovered_button(
                                                        &root_element,
                                                        &layout_tree,
                                                        point,
                                                    );
                                                    let _ = LauncherApp::update(
                                                        &mut state,
                                                        LauncherMessage::ButtonHovered(hovered_btn),
                                                    );
                                                    // Push hover events to the bus
                                                    if let Some(btn) = hovered_btn {
                                                        event_bus.push(UiEvent::Hover(
                                                            crate::core::ui::widget::ids::from_button_id(btn),
                                                        ));
                                                    } else if let Some(prev) = prev_hovered {
                                                        event_bus.push(UiEvent::HoverEnd(
                                                            crate::core::ui::widget::ids::from_button_id(prev),
                                                        ));
                                                    }
                                                    InputStatus::Handled
                                                }
                                                MotionAction::Up | MotionAction::PointerUp => {
                                                    let prev_hovered = state.hovered;
                                                    let hovered_btn = find_hovered_button(
                                                        &root_element,
                                                        &layout_tree,
                                                        point,
                                                    );
                                                    let _ = LauncherApp::update(
                                                        &mut state,
                                                        LauncherMessage::ButtonHovered(hovered_btn),
                                                    );
                                                    if let Some(prev) = prev_hovered {
                                                        event_bus.push(UiEvent::HoverEnd(
                                                            crate::core::ui::widget::ids::from_button_id(prev),
                                                        ));
                                                    }

                                                    if let Some(clicked_btn) = find_clicked_button(
                                                        &root_element,
                                                        &layout_tree,
                                                        point,
                                                    ) {
                                                        // Push Click before updating state
                                                        event_bus.push(UiEvent::Click(
                                                            crate::core::ui::widget::ids::from_button_id(clicked_btn),
                                                        ));
                                                        if let Some(action) = LauncherApp::update(
                                                            &mut state,
                                                            LauncherMessage::ButtonClicked(clicked_btn),
                                                        ) {
                                                            let _ = launch_action(action);
                                                        }
                                                    }
                                                    InputStatus::Handled
                                                }
                                                MotionAction::Cancel => {
                                                    let _ = LauncherApp::update(
                                                        &mut state,
                                                        LauncherMessage::ButtonHovered(None),
                                                    );
                                                    InputStatus::Handled
                                                }
                                                _ => InputStatus::Unhandled,
                                            }
                                        })
                                }
                                _ => InputStatus::Unhandled,
                            });

                            if !had_event {
                                break;
                            }
                        }
                    }
                }
                MainEvent::TerminateWindow { .. } => {
                    if let Some(ref mut egl) = egl_state {
                        egl.unbind();
                    }
                }
                MainEvent::Destroy => {
                    egl_state = None; // Drop EGL resources
                    running = false;
                }
                _ => {}
            },
            _ => {}
        });
    }
}
