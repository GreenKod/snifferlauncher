use crate::core::render::draw::draw_ui;
use crate::core::style::BACKGROUND;
use crate::core::ui::data_map::DataMap;
use crate::core::ui::event::EventBus;
use crate::core::ui::event::UiEvent;
use crate::core::ui::style_map::StyleMap;
use crate::core::{Action, Point, Renderer, ScreenMetrics, Size, calculate_layout};
use crate::plugin::registry::PluginRegistry;

use android_activity::{AndroidApp, InputStatus, MainEvent, PollEvent};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::window::EglContextState;

pub struct KineticScroll {
    pub sv_id: u64,
    pub velocity_x: f32,
    pub velocity_y: f32,
}

pub struct AppState {
    pub running: bool,
    pub last_touch_pos: Point,
    pub hovered_btn: Option<u64>,
    pub active_scrollview_drag: Option<u64>,
    pub last_drag_delta: (f32, f32),
    pub kinetic_scrolls: Vec<KineticScroll>,

    pub event_bus: EventBus,
    pub style_map: StyleMap,
    pub data_map: DataMap,
    pub action_queue: Arc<Mutex<Vec<Action>>>,
    pub plugin_registry: PluginRegistry,
}

impl AppState {
    #[must_use]
    pub fn new(app: &AndroidApp) -> Self {
        let mut plugin_registry = PluginRegistry::default();
        let action_queue = Arc::new(Mutex::new(Vec::new()));

        crate::plugin::PluginLoader::register_all_from_assets(
            &mut plugin_registry,
            &app.asset_manager(),
            action_queue.clone(),
        );

        Self {
            running: true,
            last_touch_pos: Point::zero(),
            hovered_btn: None,
            active_scrollview_drag: None,
            last_drag_delta: (0.0, 0.0),
            kinetic_scrolls: Vec::new(),

            event_bus: EventBus::default(),
            style_map: StyleMap::default(),
            data_map: DataMap::default(),
            action_queue,
            plugin_registry,
        }
    }
}

#[unsafe(no_mangle)]
#[allow(clippy::pedantic)]
pub fn android_main(app: AndroidApp) {
    let mut egl_state: Option<EglContextState> = None;
    let mut state = AppState::new(&app);

    let mut root_element = crate::core::types::Element::Container {
        id: None,
        style: crate::core::style::Style::default(),
        children: vec![],
    };

    while state.running {
        app.poll_events(Some(Duration::from_millis(16)), |event| match event {
            PollEvent::Wake | PollEvent::Timeout => {
                if let Some(ref mut egl) = egl_state
                    && let Some(ref mut renderer) = egl.renderer
                    && let Some(window) = app.native_window()
                {
                    state.plugin_registry.tick();

                    root_element = state.plugin_registry.build_ui().unwrap_or_else(|| {
                        crate::core::types::Element::Container {
                            id: None,
                            style: crate::core::style::Style::default(),
                            children: vec![],
                        }
                    });

                    let width =
                        f32::from(u16::try_from(window.width()).expect("window width fits in u16"));
                    let height = f32::from(
                        u16::try_from(window.height()).expect("window height fits in u16"),
                    );

                    crate::core::types::SCREEN_WIDTH
                        .store(width.to_bits(), std::sync::atomic::Ordering::Relaxed);
                    crate::core::types::SCREEN_HEIGHT
                        .store(height.to_bits(), std::sync::atomic::Ordering::Relaxed);

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

                    let (density, scaled_density) = crate::platform::android::jni::get_density();
                    let metrics = ScreenMetrics::from_scale(
                        width,
                        height - safe_area_top - safe_area_bottom,
                        density,
                        scaled_density,
                    );

                    let layout_tree = calculate_layout(
                        &root_element,
                        Size::new(width, height - safe_area_top - safe_area_bottom),
                        0.0,
                        safe_area_top,
                    );

                    state.kinetic_scrolls.retain_mut(|k| {
                        if k.velocity_x.abs() > 0.1 || k.velocity_y.abs() > 0.1 {
                            state.event_bus.push(UiEvent::Scroll(
                                Some(k.sv_id),
                                k.velocity_x,
                                k.velocity_y,
                            ));
                            k.velocity_x *= 0.92;
                            k.velocity_y *= 0.92;
                            true
                        } else {
                            false
                        }
                    });

                    state.plugin_registry.dispatch(
                        &state.event_bus,
                        &state.style_map,
                        &state.data_map,
                        &state.action_queue,
                    );

                    if let Ok(mut q) = state.action_queue.lock() {
                        for action in q.drain(..) {
                            match action {
                                crate::core::Action::LoadImage { id, src } => {
                                    if let Ok(img) = image::open(&src) {
                                        let rgba = img.to_rgba8();
                                        let (w, h) = rgba.dimensions();
                                        renderer.load_image(&id, rgba.as_raw(), w, h);
                                    }
                                }
                                crate::core::Action::FocusTextInput(_id) => {
                                    app.show_soft_input(true);
                                }
                                crate::core::Action::BlurTextInput => {
                                    app.hide_soft_input(true);
                                }
                                _ => {
                                    let _ = crate::platform::android::jni::intent::launch_action(
                                        action,
                                    );
                                }
                            }
                        }
                    }

                    renderer.begin_frame(width, height);
                    renderer.clear(BACKGROUND);

                    draw_ui(
                        renderer,
                        &root_element,
                        &layout_tree,
                        &metrics,
                        &state.style_map,
                        &state.data_map,
                        1.0,
                    );

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
                            let had_event = iter.next(|input_event| {
                                let window = app.native_window();
                                window.map_or(InputStatus::Unhandled, |win| {
                                    let width = f32::from(u16::try_from(win.width()).unwrap_or(0));
                                    let height =
                                        f32::from(u16::try_from(win.height()).unwrap_or(0));
                                    let (safe_area_top, safe_area_bottom) =
                                        crate::platform::android::jni::get_safe_area(&app)
                                            .map(|(top, bottom)| {
                                                (
                                                    f32::from(i16::try_from(top).unwrap_or(0)),
                                                    f32::from(i16::try_from(bottom).unwrap_or(0)),
                                                )
                                            })
                                            .unwrap_or((0.0, 0.0));

                                    let layout_tree = calculate_layout(
                                        &root_element,
                                        Size::new(width, height - safe_area_top - safe_area_bottom),
                                        0.0,
                                        safe_area_top,
                                    );
                                    super::input::handle_input_event(
                                        input_event,
                                        &mut state,
                                        &root_element,
                                        &layout_tree,
                                    )
                                })
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
                    egl_state = None;
                    state.running = false;
                }
                _ => {}
            },
            _ => {}
        });
    }
}
