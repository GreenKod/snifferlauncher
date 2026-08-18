use sniffer_render::draw::{draw_ui, find_clicked_button_with_scroll, find_hovered_button};
use sniffer_core::style::BACKGROUND;
use sniffer_core::ui::data_map::DataMap;
use sniffer_core::ui::event::{EventBus, UiEvent};
use sniffer_core::ui::style_map::StyleMap;
use sniffer_core::{Action, Point, Renderer, ScreenMetrics, Size, calculate_layout};
use sniffer_render::GlowRenderer;
use sniffer_plugin::registry::PluginRegistry;

use sniffer_core::{dev_err, dev_log};
use obfstr::obfstr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::window::DesktopWindow;

pub struct KineticScroll {
    pub sv_id: u64,
    pub velocity_x: f32,
    pub velocity_y: f32,
}

#[derive(Default)]
pub struct FrameInputState {
    pub clicked_pos: Option<Point>,
    pub mouse_released: bool,
    pub mouse_moved: bool,
    pub scroll_events: Vec<(f32, f32)>,
    pub drag_events: Vec<(f32, f32)>,
}

pub struct AppState {
    pub running: bool,
    pub window_focused: bool,
    pub is_mouse_down: bool,
    pub focused_input_id: Option<String>,
    pub last_mouse_pos: Point,
    pub hovered_btn: Option<u64>,
    pub active_scrollview_drag: Option<u64>,
    pub last_drag_delta: (f32, f32),
    pub kinetic_scrolls: Vec<KineticScroll>,

    pub event_bus: EventBus,
    pub style_map: StyleMap,
    pub data_map: DataMap,
    pub action_queue: Arc<Mutex<Vec<Action>>>,
    pub plugin_registry: PluginRegistry,
    pub transition_manager: sniffer_core::anim::TransitionManager,
}

#[must_use]
pub fn detect_desktop_theme() -> sniffer_core::vault::SystemTheme {
    #[cfg(target_os = "windows")]
    {
        // Check Windows Personalize registry for AppsUseLightTheme
        use std::process::Command;
        if let Ok(output) = Command::new("reg")
            .args(["query", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize", "/v", "AppsUseLightTheme"])
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            if text.contains("0x1") {
                return sniffer_core::vault::SystemTheme::light();
            } else if text.contains("0x0") {
                return sniffer_core::vault::SystemTheme::dark();
            }
        }
    }
    sniffer_core::vault::SystemTheme::dark()
}

impl AppState {
    #[must_use]
    pub fn new() -> Self {
        let mut plugin_registry = PluginRegistry::default();
        let action_queue = Arc::new(Mutex::new(Vec::new()));

        let theme = detect_desktop_theme();
        plugin_registry.vault().update_system_theme(theme);

        if let Ok(apps) = crate::get_application_list() {
            plugin_registry.vault().update_system_apps(apps);
        }

        let candidates = [
            // 1. Next to the executable (e.g. target/debug/.plugins or target/release/.plugins)
            std::env::current_exe().ok().and_then(|p| p.parent().map(|dir| dir.join(".plugins"))),
            // 2. Parent of the profile directory (e.g. target/.plugins)
            std::env::current_exe().ok().and_then(|p| p.parent().and_then(|d| d.parent()).map(|dir| dir.join(".plugins"))),
            // 3. Project compile-time manifest dir (guarantees cargo run works from anywhere)
            option_env!("CARGO_MANIFEST_DIR").map(|dir| std::path::Path::new(dir).join(".plugins")),
            // 4. Current working directory (.plugins)
            Some(std::path::PathBuf::from(".plugins")),
        ];

        let plugins_dir = candidates
            .into_iter()
            .flatten()
            .find(|p| p.exists())
            .unwrap_or_else(|| std::path::PathBuf::from(".plugins"));

        dev_log!("Using plugins directory: {}", plugins_dir.display());

        plugin_registry
            .vault()
            .set_cache_dir(plugins_dir.join(obfstr::obfstr!(".cache")));

        sniffer_plugin::PluginLoader::new(&plugins_dir)
            .register_all(&mut plugin_registry, &action_queue);

        Self {
            running: true,
            window_focused: true,
            is_mouse_down: false,
            focused_input_id: None,
            last_mouse_pos: Point::new(-9999.0, -9999.0),
            hovered_btn: None,
            active_scrollview_drag: None,
            last_drag_delta: (0.0, 0.0),
            kinetic_scrolls: Vec::new(),

            event_bus: EventBus::default(),
            style_map: StyleMap::default(),
            data_map: DataMap::default(),
            action_queue,
            plugin_registry,
            transition_manager: sniffer_core::anim::TransitionManager::default(),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

fn find_first_scrollview<'a>(
    element: &'a sniffer_core::types::Element,
    layout: &'a sniffer_core::layout::LayoutNode,
) -> Option<(
    &'a sniffer_core::types::Element,
    &'a sniffer_core::layout::LayoutNode,
)> {
    if let sniffer_core::types::Element::ScrollView { .. } = element {
        return Some((element, layout));
    }
    if let sniffer_core::types::Element::Container { children, .. } = element {
        for (child_el, child_lay) in children.iter().zip(layout.children.iter()) {
            if let Some(res) = find_first_scrollview(child_el, child_lay) {
                return Some(res);
            }
        }
    }
    None
}

#[allow(
    clippy::missing_errors_doc,
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::needless_pass_by_value,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)]
pub fn run_loop(
    mut app: AppState,
    event_loop: winit::event_loop::EventLoop<()>,
    desktop: DesktopWindow,
    mut renderer: GlowRenderer,
) -> Result<(), String> {
    let mut last_frame_time = std::time::Instant::now();
    let mut input = FrameInputState::default();

    let (phys_w, phys_h) = desktop.drawable_size();
    let width = f32::from(u16::try_from(phys_w).unwrap_or(0));
    let height = f32::from(u16::try_from(phys_h).unwrap_or(0));
    sniffer_core::types::SCREEN_WIDTH.store(width.to_bits(), std::sync::atomic::Ordering::Relaxed);
    sniffer_core::types::SCREEN_HEIGHT.store(height.to_bits(), std::sync::atomic::Ordering::Relaxed);

    desktop.window.request_redraw();

    #[allow(deprecated)]
    event_loop
        .run(move |event, active_event_loop| match event {
            winit::event::Event::WindowEvent { event, .. } => {
                match &event {
                    winit::event::WindowEvent::CloseRequested => {
                        app.running = false;
                        active_event_loop.exit();
                        return;
                    }
                    winit::event::WindowEvent::Resized(size) => {
                        desktop.resize_surface(size.width, size.height);
                        let (phys_w, phys_h) = desktop.drawable_size();
                        let width = f32::from(u16::try_from(phys_w).unwrap_or(0));
                        let height = f32::from(u16::try_from(phys_h).unwrap_or(0));
                        sniffer_core::types::SCREEN_WIDTH
                            .store(width.to_bits(), std::sync::atomic::Ordering::Relaxed);
                        sniffer_core::types::SCREEN_HEIGHT
                            .store(height.to_bits(), std::sync::atomic::Ordering::Relaxed);

                        app.event_bus.push(UiEvent::WindowResized(
                            width,
                            height,
                        ));
                        desktop.window.request_redraw();
                    }
                    winit::event::WindowEvent::ScaleFactorChanged { .. } => {
                        desktop.window.request_redraw();
                    }
                    winit::event::WindowEvent::RedrawRequested => {
                        let now = std::time::Instant::now();
                        let dt = now.duration_since(last_frame_time).as_secs_f32();
                        last_frame_time = now;

                        let (log_w, log_h) = desktop.size();
                        let (phys_w, phys_h) = desktop.drawable_size();

                        let width = f32::from(u16::try_from(phys_w).unwrap_or(0));
                        let height = f32::from(u16::try_from(phys_h).unwrap_or(0));

                        sniffer_core::types::SCREEN_WIDTH
                            .store(width.to_bits(), std::sync::atomic::Ordering::Relaxed);
                        sniffer_core::types::SCREEN_HEIGHT
                            .store(height.to_bits(), std::sync::atomic::Ordering::Relaxed);

                        app.plugin_registry.tick();

                        let root_element = app.plugin_registry.build_ui().unwrap_or_else(|| {
                            sniffer_core::types::Element::Container {
                                id: None,
                                style: sniffer_core::style::Style::default(),
                                children: vec![],
                            }
                        });

                        app.transition_manager.sync_tree(&root_element);
                        let _ = app.transition_manager.tick(dt);

                        let scale_x = width / f32::from(u16::try_from(log_w).unwrap_or(1));
                        let scale_y = height / f32::from(u16::try_from(log_h).unwrap_or(1));

                        let mut scaled_last_mouse_pos = app.last_mouse_pos;
                        if (scaled_last_mouse_pos.x - -9999.0).abs() > f32::EPSILON {
                            scaled_last_mouse_pos.x *= scale_x;
                            scaled_last_mouse_pos.y *= scale_y;
                        }

                        let scaled_clicked_pos = input.clicked_pos.map(|mut pos| {
                            pos.x *= scale_x;
                            pos.y *= scale_y;
                            pos
                        });

                        let metrics = ScreenMetrics::from_dpi(width, height, desktop.dpi);
                        let layout_tree =
                            calculate_layout(&root_element, Size::new(width, height), 0.0, 0.0);

                        if input.mouse_moved {
                            let prev_hovered = app.hovered_btn;
                            let hovered_data = find_hovered_button(
                                &root_element,
                                &layout_tree,
                                scaled_last_mouse_pos,
                            );
                            app.hovered_btn = hovered_data.map(|(id, _)| id);

                            match (prev_hovered, hovered_data) {
                                (Some(prev), Some((current, rect))) if prev != current => {
                                    app.event_bus.push(UiEvent::HoverEnd(prev));
                                    app.event_bus.push(UiEvent::Hover(
                                        current,
                                        rect.width,
                                        rect.height,
                                        scaled_last_mouse_pos.x - rect.x,
                                        scaled_last_mouse_pos.y - rect.y,
                                    ));
                                }
                                (Some(prev), Some((current, rect))) if prev == current => {
                                    app.event_bus.push(UiEvent::Hover(
                                        current,
                                        rect.width,
                                        rect.height,
                                        scaled_last_mouse_pos.x - rect.x,
                                        scaled_last_mouse_pos.y - rect.y,
                                    ));
                                }
                                (None, Some((current, rect))) => {
                                    app.event_bus.push(UiEvent::Hover(
                                        current,
                                        rect.width,
                                        rect.height,
                                        scaled_last_mouse_pos.x - rect.x,
                                        scaled_last_mouse_pos.y - rect.y,
                                    ));
                                }
                                (Some(prev), None) => {
                                    app.event_bus.push(UiEvent::HoverEnd(prev));
                                }
                                _ => {}
                            }
                        }

                        if let Some(clicked_pt) = scaled_clicked_pos {
                            if let Some((
                                sniffer_core::types::Element::ScrollView {
                                    id, capture_drag, ..
                                },
                                _,
                            )) = sniffer_render::draw::find_hovered_scrollview(
                                &root_element,
                                &layout_tree,
                                clicked_pt,
                            ) {
                                if capture_drag.unwrap_or(true) {
                                    app.active_scrollview_drag = id.as_deref().map(|id_str| {
                                        sniffer_core::ui::widget::fnv1a(id_str.as_bytes())
                                    });
                                }
                                app.kinetic_scrolls.clear();
                            }

                            if let Some((clicked_btn, rect)) =
                                find_clicked_button_with_scroll(
                                    &root_element,
                                    &layout_tree,
                                    clicked_pt,
                                    &|_id_opt, sx, sy| (sx, sy),
                                )
                            {
                                app.event_bus.push(UiEvent::PointerDown(clicked_btn));
                                app.event_bus.push(UiEvent::Click(
                                    clicked_btn,
                                    rect.width,
                                    rect.height,
                                ));
                            } else {
                                app.event_bus.push(UiEvent::PointerDown(0));
                                app.event_bus.push(UiEvent::ClickOutside);
                            }
                        }

                        if input.mouse_released {
                            if let Some(sv_id) = app.active_scrollview_drag
                                && (app.last_drag_delta.0.abs() > 0.5
                                    || app.last_drag_delta.1.abs() > 0.5)
                            {
                                let momentum_enabled = if let Some((
                                    sniffer_core::types::Element::ScrollView {
                                        momentum_scrolling,
                                        ..
                                    },
                                    _,
                                )) =
                                    sniffer_render::draw::find_hovered_scrollview(
                                        &root_element,
                                        &layout_tree,
                                        scaled_last_mouse_pos,
                                    ) {
                                    momentum_scrolling.unwrap_or(true)
                                } else {
                                    true
                                };

                                if momentum_enabled {
                                    app.kinetic_scrolls.push(KineticScroll {
                                        sv_id,
                                        velocity_x: app.last_drag_delta.0,
                                        velocity_y: app.last_drag_delta.1,
                                    });
                                }
                            }

                            let released_btn = find_hovered_button(
                                &root_element,
                                &layout_tree,
                                scaled_last_mouse_pos,
                            )
                            .map(|(id, _)| id);
                            app.event_bus.push(UiEvent::PointerUp(released_btn));

                            app.active_scrollview_drag = None;
                            app.last_drag_delta = (0.0, 0.0);
                        }

                        let get_max_scroll_for_lay =
                            |lay: &sniffer_core::layout::LayoutNode| -> (f32, f32) {
                                let view_width = lay.rect.width;
                                let view_height = lay.rect.height;
                                let mut max_x = 0.0_f32;
                                let mut max_y = 0.0_f32;
                                for child_lay in &lay.children {
                                    let child_right = child_lay.rect.x + child_lay.rect.width;
                                    let child_bottom = child_lay.rect.y + child_lay.rect.height;
                                    if child_right > max_x {
                                        max_x = child_right;
                                    }
                                    if child_bottom > max_y {
                                        max_y = child_bottom;
                                    }
                                }
                                let content_width = max_x - lay.rect.x;
                                let content_height = max_y - lay.rect.y;

                                let max_scroll_x =
                                    if content_width > view_width && view_width > 0.0 {
                                        content_width - view_width
                                    } else {
                                        0.0
                                    };
                                let max_scroll_y =
                                    if content_height > view_height && view_height > 0.0 {
                                        content_height - view_height
                                    } else {
                                        0.0
                                    };
                                (max_scroll_x, max_scroll_y)
                            };

                        for &(x, y) in &input.scroll_events {
                            let target = sniffer_render::draw::find_hovered_scrollview(
                                &root_element,
                                &layout_tree,
                                scaled_last_mouse_pos,
                            )
                            .or_else(|| find_first_scrollview(&root_element, &layout_tree));

                            if let Some((
                                sniffer_core::types::Element::ScrollView {
                                    id,
                                    scroll_sensitivity,
                                    dynamic_sensitivity,
                                    ..
                                },
                                lay,
                            )) = target
                            {
                                let mut factor = scroll_sensitivity.unwrap_or(1.0);
                                if dynamic_sensitivity.unwrap_or(false) && !lay.children.is_empty()
                                {
                                    let view_height = lay.rect.height;
                                    let mut min_y = f32::MAX;
                                    let mut max_y = f32::MIN;
                                    for child in &lay.children {
                                        if child.rect.y < min_y {
                                            min_y = child.rect.y;
                                        }
                                        if child.rect.y + child.rect.height > max_y {
                                            max_y = child.rect.y + child.rect.height;
                                        }
                                    }
                                    let content_height = max_y - min_y;
                                    if view_height > 0.0 && content_height > view_height {
                                        factor *= content_height / view_height;
                                    }
                                }

                                let sv_id = id.as_deref().map(|id_str| {
                                    sniffer_core::ui::widget::fnv1a(id_str.as_bytes())
                                });

                                let max_scroll = get_max_scroll_for_lay(lay);

                                app.event_bus.push(UiEvent::Scroll(
                                    sv_id,
                                    -x * 20.0 * factor,
                                    -y * 20.0 * factor,
                                    max_scroll.0,
                                    max_scroll.1,
                                ));
                            }
                        }

                        let mut total_dx = 0.0;
                        let mut total_dy = 0.0;
                        for &(dx, dy) in &input.drag_events {
                            let s_dx = -dx * scale_x;
                            let s_dy = -dy * scale_y;
                            total_dx += s_dx;
                            total_dy += s_dy;

                            let target = sniffer_render::draw::find_hovered_scrollview(
                                &root_element,
                                &layout_tree,
                                scaled_last_mouse_pos,
                            )
                            .or_else(|| find_first_scrollview(&root_element, &layout_tree));

                            if let Some(sv_id) = app.active_scrollview_drag {
                                if let Some((_, lay)) = target {
                                    let max_scroll = get_max_scroll_for_lay(lay);
                                    app.event_bus.push(UiEvent::Scroll(
                                        Some(sv_id),
                                        s_dx,
                                        s_dy,
                                        max_scroll.0,
                                        max_scroll.1,
                                    ));
                                }
                            } else if let Some((
                                sniffer_core::types::Element::ScrollView {
                                    id, capture_drag, ..
                                },
                                lay,
                            )) = target
                            {
                                if capture_drag.unwrap_or(true)
                                    && app.active_scrollview_drag.is_none()
                                {
                                    app.active_scrollview_drag = id.as_deref().map(|id_str| {
                                        sniffer_core::ui::widget::fnv1a(id_str.as_bytes())
                                    });
                                }
                                let sv_id = id.as_deref().map(|id_str| {
                                    sniffer_core::ui::widget::fnv1a(id_str.as_bytes())
                                });
                                let max_scroll = get_max_scroll_for_lay(lay);
                                app.event_bus.push(UiEvent::Scroll(
                                    sv_id,
                                    s_dx,
                                    s_dy,
                                    max_scroll.0,
                                    max_scroll.1,
                                ));
                            }
                        }

                        if total_dx != 0.0 || total_dy != 0.0 {
                            app.last_drag_delta = (total_dx, total_dy);
                        }

                        app.kinetic_scrolls.retain_mut(|k| {
                            if k.velocity_x.abs() > 0.1 || k.velocity_y.abs() > 0.1 {
                                let max_scroll = find_first_scrollview(&root_element, &layout_tree)
                                    .map_or((0.0, 0.0), |(_, lay)| {
                                        get_max_scroll_for_lay(lay)
                                    });
                                app.event_bus.push(UiEvent::Scroll(
                                    Some(k.sv_id),
                                    k.velocity_x,
                                    k.velocity_y,
                                    max_scroll.0,
                                    max_scroll.1,
                                ));
                                k.velocity_x *= 0.92;
                                k.velocity_y *= 0.92;
                                true
                            } else {
                                false
                            }
                        });

                        app.plugin_registry.dispatch(
                            &app.event_bus,
                            &app.style_map,
                            &app.data_map,
                            &app.action_queue,
                        );

                        if let Ok(mut q) = app.action_queue.lock() {
                            for action in q.drain(..) {
                                match action {
                                    Action::OpenSettings => {
                                        dev_log!("{}", obfstr!("Desktop Preview: Open Settings triggered"));
                                    }
                                    Action::OpenContacts => {
                                        dev_log!("{}", obfstr!("Desktop Preview: Open Contacts triggered"));
                                    }
                                    Action::OpenCamera => {
                                        dev_log!("{}", obfstr!("Desktop Preview: Open Camera triggered"));
                                    }
                                    Action::LoadImage { id, src } => {
                                        if src.starts_with(obfstr!("app-icon://")) {
                                            let w = 64;
                                            let h = 64;
                                            let pixels = [0, 255, 0, 255].repeat((w * h) as usize);
                                            renderer.load_image(&id, &pixels, w, h);
                                            dev_log!("{} {src}", obfstr!("Loaded dummy app icon for"));
                                        } else {
                                            let result = image::open(&src);
                                            match result {
                                                Ok(img) => {
                                                    let rgba = img.to_rgba8();
                                                    let (w, h) = rgba.dimensions();
                                                    renderer.load_image(&id, rgba.as_raw(), w, h);
                                                }
                                                Err(e) => dev_err!("{}: {e}", obfstr!("Failed to load image")),
                                            }
                                        }
                                    }
                                    Action::LaunchApp { package_name } => {
                                        dev_log!("{}: {package_name}", obfstr!("Desktop Preview: Launch App triggered"));
                                    }
                                    Action::FocusTextInput(id) => {
                                        desktop.window.set_ime_allowed(true);
                                        app.focused_input_id = Some(id);
                                    }
                                    Action::RequestDefaultLauncher => {
                                        dev_log!("{}", obfstr!("Desktop Preview: Request Default Launcher triggered"));
                                    }
                                    Action::BlurTextInput => {
                                        desktop.window.set_ime_allowed(false);
                                        app.focused_input_id = None;
                                    }
                                }
                            }
                        }

                        renderer.begin_frame(width, height);
                        renderer.clear(BACKGROUND);

                        draw_ui(
                            &mut renderer,
                            &root_element,
                            &layout_tree,
                            &metrics,
                            &app.style_map,
                            &app.data_map,
                            &app.transition_manager,
                            1.0,
                            0.0,
                            0.0,
                        );

                        renderer.end_frame();
                        let _ = desktop.swap_buffers();

                        input = FrameInputState::default();
                    }
                    _ => {}
                }

                super::input::handle_winit_event(&event, &mut app, &mut input);
                desktop.window.request_redraw();
            }
            winit::event::Event::AboutToWait => {
                if !app.running {
                    active_event_loop.exit();
                    return;
                }

                desktop.window.request_redraw();
                active_event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(
                    std::time::Instant::now() + Duration::from_millis(16),
                ));
            }
            _ => {}
        })
        .map_err(|e| e.to_string())?;

    Ok(())
}

