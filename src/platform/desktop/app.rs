use crate::core::render::draw::{draw_ui, find_clicked_button, find_hovered_button};
use crate::core::style::BACKGROUND;
use crate::core::ui::data_map::DataMap;
use crate::core::ui::event::{EventBus, UiEvent};
use crate::core::ui::style_map::StyleMap;
use crate::core::{Action, GlowRenderer, Point, Renderer, ScreenMetrics, Size, calculate_layout};
use crate::plugin::registry::PluginRegistry;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::window::DesktopWindow;

pub struct KineticScroll {
    pub sv_id: u64,
    pub velocity_x: f32,
    pub velocity_y: f32,
}

pub struct AppState {
    pub running: bool,
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
    pub transition_manager: crate::core::anim::TransitionManager,
}

impl AppState {
    #[must_use]
    pub fn new() -> Self {
        let mut plugin_registry = PluginRegistry::default();
        let action_queue = Arc::new(Mutex::new(Vec::new()));

        let mut plugins_dir = std::env::current_exe()
            .unwrap_or_default()
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join(".plugins");

        if !plugins_dir.exists() {
            // Fallback to current working directory
            plugins_dir = std::path::PathBuf::from(".plugins");
        }

        crate::plugin::PluginLoader::new(&plugins_dir)
            .register_all(&mut plugin_registry, &action_queue);

        Self {
            running: true,
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
            transition_manager: crate::core::anim::TransitionManager::default(),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(
    clippy::missing_errors_doc,
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::needless_pass_by_value
)]
pub fn run_loop(
    mut app: AppState,
    desktop: DesktopWindow,
    mut renderer: GlowRenderer,
) -> Result<(), String> {
    let mut event_pump = desktop.sdl_context.event_pump()?;
    let mut last_frame_time = std::time::Instant::now();

    while app.running {
        let now = std::time::Instant::now();
        let dt = now.duration_since(last_frame_time).as_secs_f32();
        last_frame_time = now;

        app.plugin_registry.tick();

        let root_element = app.plugin_registry.build_ui().unwrap_or_else(|| {
            crate::core::types::Element::Container {
                id: None,
                style: crate::core::style::Style::default(),
                children: vec![],
            }
        });

        let mut clicked_pos = None;
        let mut mouse_released = false;
        let mut mouse_moved = false;
        let mut scroll_events = Vec::new();
        let mut drag_events = Vec::new();

        app.transition_manager.sync_tree(&root_element);
        let _anim_needs_redraw = app.transition_manager.tick(dt);

        for event in event_pump.poll_iter() {
            super::input::handle_event(
                event,
                &mut app,
                &mut clicked_pos,
                &mut mouse_released,
                &mut mouse_moved,
                &mut scroll_events,
                &mut drag_events,
            );
        }

        let (log_w, log_h) = desktop.window.size();
        let (phys_w, phys_h) = desktop.window.drawable_size();

        let width = f32::from(u16::try_from(phys_w).unwrap_or(0));
        let height = f32::from(u16::try_from(phys_h).unwrap_or(0));

        crate::core::types::SCREEN_WIDTH
            .store(width.to_bits(), std::sync::atomic::Ordering::Relaxed);
        crate::core::types::SCREEN_HEIGHT
            .store(height.to_bits(), std::sync::atomic::Ordering::Relaxed);

        let scale_x = width / f32::from(u16::try_from(log_w).unwrap_or(1));
        let scale_y = height / f32::from(u16::try_from(log_h).unwrap_or(1));

        let mut scaled_last_mouse_pos = app.last_mouse_pos;
        if (scaled_last_mouse_pos.x - -9999.0).abs() > f32::EPSILON {
            scaled_last_mouse_pos.x *= scale_x;
            scaled_last_mouse_pos.y *= scale_y;
        }

        let scaled_clicked_pos = clicked_pos.map(|mut pos| {
            pos.x *= scale_x;
            pos.y *= scale_y;
            pos
        });

        let metrics = ScreenMetrics::from_dpi(width, height, desktop.dpi);
        let layout_tree = calculate_layout(&root_element, Size::new(width, height), 0.0, 0.0);

        if mouse_moved {
            let prev_hovered = app.hovered_btn;
            let hovered_data =
                find_hovered_button(&root_element, &layout_tree, scaled_last_mouse_pos);
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
                crate::core::types::Element::ScrollView {
                    id, capture_drag, ..
                },
                _,
            )) = crate::core::render::draw::find_hovered_scrollview(
                &root_element,
                &layout_tree,
                clicked_pt,
            ) {
                if capture_drag.unwrap_or(true) {
                    app.active_scrollview_drag = id
                        .as_deref()
                        .map(|id_str| crate::core::ui::widget::fnv1a(id_str.as_bytes()));
                }
                app.kinetic_scrolls.clear();
            }

            if let Some((clicked_btn, rect)) =
                find_clicked_button(&root_element, &layout_tree, clicked_pt)
            {
                app.event_bus.push(UiEvent::PointerDown(clicked_btn));
                app.event_bus
                    .push(UiEvent::Click(clicked_btn, rect.width, rect.height));
            } else {
                app.event_bus.push(UiEvent::PointerDown(0));
                app.event_bus.push(UiEvent::ClickOutside);
            }
        }

        if mouse_released {
            if let Some(sv_id) = app.active_scrollview_drag
                && (app.last_drag_delta.0.abs() > 0.5 || app.last_drag_delta.1.abs() > 0.5)
            {
                let momentum_enabled = if let Some((
                    crate::core::types::Element::ScrollView {
                        momentum_scrolling, ..
                    },
                    _,
                )) = crate::core::render::draw::find_hovered_scrollview(
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

            let released_btn =
                find_hovered_button(&root_element, &layout_tree, scaled_last_mouse_pos)
                    .map(|(id, _)| id);
            app.event_bus.push(UiEvent::PointerUp(released_btn));

            app.active_scrollview_drag = None;
            app.last_drag_delta = (0.0, 0.0);
        }

        let get_max_scroll = |target_id: Option<u64>| -> f32 {
            target_id.map_or(0.0, |target| {
                let mut found_max = 0.0;
                let mut search = vec![(&root_element, &layout_tree)];
                while let Some((el, lay)) = search.pop() {
                    if let crate::core::types::Element::ScrollView { id, .. } = el
                        && id
                            .as_deref()
                            .map(|s| crate::core::ui::widget::fnv1a(s.as_bytes()))
                            == Some(target)
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
                        if min_y <= max_y {
                            found_max = (max_y - min_y - view_height).max(0.0);
                        }
                        break;
                    }
                    if let crate::core::types::Element::Container { children, .. }
                    | crate::core::types::Element::ScrollView { children, .. } = el
                    {
                        for (child, child_lay) in children.iter().zip(lay.children.iter()) {
                            search.push((child, child_lay));
                        }
                    }
                }
                found_max
            })
        };

        for (x, y) in scroll_events {
            if let Some((
                crate::core::types::Element::ScrollView {
                    id,
                    scroll_sensitivity,
                    dynamic_sensitivity,
                    ..
                },
                lay,
            )) = crate::core::render::draw::find_hovered_scrollview(
                &root_element,
                &layout_tree,
                scaled_last_mouse_pos,
            ) {
                let mut factor = scroll_sensitivity.unwrap_or(1.0);
                if dynamic_sensitivity.unwrap_or(false) && !lay.children.is_empty() {
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

                let sv_id = id
                    .as_deref()
                    .map(|id_str| crate::core::ui::widget::fnv1a(id_str.as_bytes()));
                app.event_bus.push(UiEvent::Scroll(
                    sv_id,
                    f32::from(i16::try_from(x).unwrap_or(0)) * -20.0 * factor,
                    f32::from(i16::try_from(y).unwrap_or(0)) * -20.0 * factor,
                    get_max_scroll(sv_id),
                ));
            }
        }

        let mut total_dx = 0.0;
        let mut total_dy = 0.0;
        for (dx, dy) in drag_events {
            let s_dx = f32::from(i16::try_from(dx).unwrap_or(0)) * scale_x;
            let s_dy = f32::from(i16::try_from(dy).unwrap_or(0)) * scale_y;
            total_dx += s_dx;
            total_dy += s_dy;

            if let Some(sv_id) = app.active_scrollview_drag {
                app.event_bus.push(UiEvent::Scroll(
                    Some(sv_id),
                    s_dx,
                    s_dy,
                    get_max_scroll(Some(sv_id)),
                ));
            } else if let Some((
                crate::core::types::Element::ScrollView {
                    id, capture_drag, ..
                },
                _,
            )) = crate::core::render::draw::find_hovered_scrollview(
                &root_element,
                &layout_tree,
                scaled_last_mouse_pos,
            ) {
                if capture_drag.unwrap_or(true) && app.active_scrollview_drag.is_none() {
                    app.active_scrollview_drag = id
                        .as_deref()
                        .map(|id_str| crate::core::ui::widget::fnv1a(id_str.as_bytes()));
                }
                let sv_id = id
                    .as_deref()
                    .map(|id_str| crate::core::ui::widget::fnv1a(id_str.as_bytes()));
                app.event_bus
                    .push(UiEvent::Scroll(sv_id, s_dx, s_dy, get_max_scroll(sv_id)));
            }
        }

        if total_dx != 0.0 || total_dy != 0.0 {
            app.last_drag_delta = (total_dx, total_dy);
        }

        app.kinetic_scrolls.retain_mut(|k| {
            if k.velocity_x.abs() > 0.1 || k.velocity_y.abs() > 0.1 {
                app.event_bus.push(UiEvent::Scroll(
                    Some(k.sv_id),
                    k.velocity_x,
                    k.velocity_y,
                    get_max_scroll(Some(k.sv_id)),
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
                    Action::OpenSettings => println!("Desktop Preview: Open Settings triggered"),
                    Action::OpenContacts => println!("Desktop Preview: Open Contacts triggered"),
                    Action::OpenCamera => println!("Desktop Preview: Open Camera triggered"),
                    Action::LoadImage { id, src } => {
                        if src.starts_with("app-icon://") {
                            // Dummy 64x64 green image for desktop simulation
                            let w = 64;
                            let h = 64;
                            let pixels = [0, 255, 0, 255].repeat((w * h) as usize);
                            renderer.load_image(&id, &pixels, w, h);
                            println!("Loaded dummy app icon for {src}");
                        } else {
                            let result = image::open(&src);
                            match result {
                                Ok(img) => {
                                    let rgba = img.to_rgba8();
                                    let (w, h) = rgba.dimensions();
                                    renderer.load_image(&id, rgba.as_raw(), w, h);
                                }
                                Err(e) => eprintln!("Failed to load image: {e}"),
                            }
                        }
                    }
                    Action::FocusTextInput(_) => {
                        desktop.video_subsystem.text_input().start();
                    }
                    Action::BlurTextInput => {
                        desktop.video_subsystem.text_input().stop();
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
        );

        renderer.end_frame();
        desktop.window.gl_swap_window();

        std::thread::sleep(Duration::from_millis(16));
    }

    Ok(())
}
