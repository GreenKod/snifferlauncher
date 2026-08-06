use crate::core::render::draw::draw_ui;
use crate::core::style::BACKGROUND;
use crate::core::ui::data_map::DataMap;
use crate::core::ui::event::EventBus;
use crate::core::ui::event::UiEvent;
use crate::core::ui::style_map::StyleMap;
use crate::core::{Action, Point, Renderer, ScreenMetrics, Size, calculate_layout};
use crate::plugin::registry::PluginRegistry;

use crate::{dev_err, dev_log};
use android_activity::{AndroidApp, InputStatus, MainEvent, PollEvent};
use obfstr::obfstr;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::window::EglContextState;

pub struct KineticScroll {
    pub sv_id: u64,
    pub velocity_x: f32,
    pub velocity_y: f32,
}

/// Rust tarafında yönetilen her ScrollView için fizik durumu.
/// Eklenti yazarı bunlara hiç dokunmaz; Rust otomatik yönetir.
pub struct ScrollPhysics {
    /// Mevcut kaydırma konumu (px)
    pub pos_x: f32,
    pub pos_y: f32,
    /// Momentum hızı (px/sn)
    pub vel_x: f32,
    pub vel_y: f32,
    /// Parmak sürüklüyor mu?
    pub is_dragging: bool,

    /// Native Android Mass-Spring-Damper durumları
    pub snap_target_x: Option<f32>,
    pub spring_start_x: f32,
    pub spring_start_vel_x: f32,
    pub spring_time: f32,

    /// Bir önceki snap'te tamamlanan sayfa
    pub last_snap_page: i32,
    /// Bu frame snap tamamlandı mı?
    pub snap_just_completed: bool,

    // ── Config (element tree'den her frame okunur) ──
    pub snap_x: Option<f32>,
    pub rubber_band: Option<f32>,
    pub page_count: Option<u32>,
    pub on_snap: Option<String>,
}

pub const RUBBER_BAND_COEFF: f32 = 0.55;

/// iOS UIScrollView asymptotic logarithmic rubber band formula.
///
/// Converts a raw boundary overscroll distance (`overscroll`) into a damped
/// visual position that asymptotically approaches the viewport dimension (`dimension`).
/// `coeff` is the damping coefficient (iOS standard = 0.55).
#[must_use]
pub fn rubber_band_clamp(overscroll: f32, dimension: f32, coeff: f32) -> f32 {
    let d = dimension.max(1.0);
    let abs_over = overscroll.abs();
    let damped = d * (1.0 - 1.0 / (coeff * (abs_over / d) + 1.0));
    damped * overscroll.signum()
}

impl ScrollPhysics {
    /// Fizik motorunu dt saniye ilerletir. Redraw gerekiyorsa true döner.
    pub fn tick(&mut self, dt: f32) -> bool {
        let mut active = false;
        self.snap_just_completed = false;

        let max_limit = self
            .snap_x
            .zip(self.page_count)
            .map(|(s, n)| s * (n as f32 - 1.0))
            .unwrap_or(f32::MAX);

        if let Some(target) = self.snap_target_x {
            // Native Android System UI Mass-Spring-Damper (Critically Damped, ζ = 1.0)
            self.spring_time += dt;
            let t = self.spring_time;
            let y0 = self.spring_start_x - target;
            let v0 = self.spring_start_vel_x;

            let omega = 16.0_f32; // ω = 16 rad/s — Android 14 Jetpack Spring (en iyi snappy akıcılık)
            let exp_term = (-omega * t).exp();

            let pos = target + (y0 + (v0 + omega * y0) * t) * exp_term;
            let vel = (v0 - omega * (y0 + (v0 + omega * y0) * t)) * exp_term;

            self.pos_x = pos;
            self.vel_x = vel;
            active = true;

            if max_limit < f32::MAX {
                if target == max_limit && self.pos_x > max_limit {
                    self.pos_x = max_limit;
                } else if target == 0.0 && self.pos_x < 0.0 {
                    self.pos_x = 0.0;
                }
            }

            // Kilitlenme tamamlandı mı? eşik 0.5px → daha hızlı oturma
            if exp_term < 0.001 || (self.pos_x - target).abs() < 0.5 {
                self.pos_x = target;
                self.vel_x = 0.0;
                self.snap_target_x = None;
                self.spring_time = 0.0;
                self.snap_just_completed = true;
                active = false;
            }
        } else if self.vel_x.abs() > 0.5 {
            // Frame-rate independent kinetic momentum
            // Doğru formül: decay = e^(-lambda * dt), lambda = -ln(friction_per_frame) * fps
            // 0.88 per frame at 60fps → lambda = -ln(0.88) * 60 ≈ 7.62
            let lambda = 7.62_f32; // = -ln(0.88) * 60
            let decay = (-lambda * dt).exp();
            self.pos_x += self.vel_x * dt;
            self.vel_x *= decay;
            active = true;

            // Sıkı sınır kinetic kaydırmada da
            if self.pos_x < 0.0 {
                self.pos_x = 0.0;
                self.vel_x = 0.0;
            } else if max_limit < f32::MAX && self.pos_x > max_limit {
                self.pos_x = max_limit;
                self.vel_x = 0.0;
            }
        }

        active || self.snap_just_completed
    }

    pub fn apply_drag(&mut self, delta_x: f32) {
        let max_x = self
            .snap_x
            .zip(self.page_count)
            .map(|(s, n)| s * (n as f32 - 1.0))
            .unwrap_or(f32::MAX);

        let viewport_w = self.snap_x.unwrap_or(1080.0);
        let coeff = self.rubber_band.unwrap_or(RUBBER_BAND_COEFF);

        let new_pos = self.pos_x + delta_x;
        if new_pos < 0.0 {
            // İlk sayfa öncesi: iOS logaritmik rubber band
            self.pos_x = rubber_band_clamp(new_pos, viewport_w, coeff);
        } else if new_pos > max_x && max_x < f32::MAX / 2.0 {
            // Son sayfa sonrası: iOS logaritmik rubber band
            let overscroll = new_pos - max_x;
            self.pos_x = max_x + rubber_band_clamp(overscroll, viewport_w, coeff);
        } else {
            self.pos_x = new_pos;
        }
    }

    /// Dokunma sonlandırıldığında sürüklemeyi bırakır ve hıza göre snap hedefini hesaplar.
    pub fn release_drag(&mut self, vel_x: f32) {
        self.is_dragging = false;
        self.vel_x = vel_x;

        let max_x = self
            .snap_x
            .zip(self.page_count)
            .map(|(s, n)| s * (n as f32 - 1.0))
            .unwrap_or(f32::MAX);

        // pos_x her zaman geçerli aralıkta olmalı (apply_drag garantiler ama çift kontrol)
        if max_x < f32::MAX {
            self.pos_x = self.pos_x.clamp(0.0, max_x);
        }

        if let Some(page_width) = self.snap_x {
            if page_width > 0.0 {
                let last_page = (self.page_count.unwrap_or(1) as f32) - 1.0;
                let page_float = self.pos_x / page_width;

                let target_page = if vel_x > 80.0 {
                    // İleri yönlü swipe (sol→sağ kaydırma) → sonraki sayfaya snap
                    (page_float.floor() + 1.0).min(last_page)
                } else if vel_x < -80.0 {
                    // Geri yönlü swipe → önceki sayfaya snap
                    (page_float.ceil() - 1.0).max(0.0)
                } else {
                    // Yavaş bırakma → %50 eşiği ile en yakın sayfa
                    page_float.round().clamp(0.0, last_page)
                };

                let target_x = (target_page * page_width).clamp(0.0, max_x);
                self.snap_target_x = Some(target_x);
                self.spring_start_x = self.pos_x; // pos_x artık max_x'i aşmıyor

                // Sınır kilitlenmesinde dışa doğru hızı sıfırla (sektirme/flicker engelleme)
                let mut init_vel = self.vel_x;
                if target_x >= max_x && init_vel > 0.0 {
                    init_vel = 0.0;
                } else if target_x <= 0.0 && init_vel < 0.0 {
                    init_vel = 0.0;
                }
                self.spring_start_vel_x = init_vel;
                self.spring_time = 0.0;
            }
        }
    }
}

pub struct AppState {
    pub running: bool,
    pub last_touch_pos: Point,
    pub hovered_btn: Option<u64>,
    pub active_scrollview_drag: Option<u64>,
    pub last_drag_delta: (f32, f32),
    /// Rolling velocity history for smooth momentum calculation
    pub drag_history: VecDeque<(f32, f32, std::time::Instant)>,
    pub kinetic_scrolls: Vec<KineticScroll>,
    /// Rust-owned scroll physics per widget ID (FNV-1a)
    pub scroll_physics: std::collections::HashMap<u64, ScrollPhysics>,

    pub event_bus: EventBus,
    pub style_map: StyleMap,
    pub data_map: DataMap,
    pub action_queue: Arc<Mutex<Vec<Action>>>,
    pub plugin_registry: PluginRegistry,
    pub transition_manager: crate::core::anim::TransitionManager,

    pub cached_safe_area: (f32, f32),
    pub cached_density: (f32, f32),
    pub cached_screen_size: (f32, f32),
    pub cached_layout: Option<Arc<crate::core::layout::LayoutNode>>,
    pub cached_max_scroll: std::collections::HashMap<u64, f32>,
    pub last_ui_version: u64,
    pub virtual_page_manager: crate::core::virtualization::VirtualPageManager,
    pub total_touch_drag_distance: f32,
}

impl AppState {
    #[must_use]
    pub fn new(app: &AndroidApp) -> Self {
        let mut plugin_registry = PluginRegistry::default();
        let action_queue = Arc::new(Mutex::new(Vec::new()));

        crate::plugin::PluginLoader::register_all_from_assets(
            &mut plugin_registry,
            &app.asset_manager(),
            &action_queue,
        );

        Self {
            running: true,
            last_touch_pos: Point::zero(),
            hovered_btn: None,
            active_scrollview_drag: None,
            last_drag_delta: (0.0, 0.0),
            drag_history: VecDeque::new(),
            kinetic_scrolls: Vec::new(),
            scroll_physics: std::collections::HashMap::new(),

            event_bus: EventBus::default(),
            style_map: StyleMap::default(),
            data_map: DataMap::default(),
            action_queue,
            plugin_registry,
            transition_manager: crate::core::anim::TransitionManager::default(),

            cached_safe_area: (0.0, 0.0),
            cached_density: (1.0, 1.0),
            cached_screen_size: (0.0, 0.0),
            cached_layout: None,
            cached_max_scroll: std::collections::HashMap::new(),
            last_ui_version: 0,
            virtual_page_manager: crate::core::virtualization::VirtualPageManager::new(),
            total_touch_drag_distance: 0.0,
        }
    }
}

/// Element tree'den ScrollPhysics config alanlar\u0131n\u0131 senkronize eder.
/// pos_x, vel_x gibi dinamik fizik de\u011ferleri KORUNUR, sadece snap_x, page_count vs. g\u00fcncellenir.
fn sync_scroll_physics_from_tree(
    element: &crate::core::types::Element,
    physics: &mut std::collections::HashMap<u64, ScrollPhysics>,
) {
    if let crate::core::types::Element::ScrollView {
        id,
        snap_x,
        snap_y,
        rubber_band,
        page_count,
        on_snap,
        ..
    } = element
    {
        if snap_x.is_some() || snap_y.is_some() {
            if let Some(id_str) = id {
                let wid = crate::core::ui::widget::fnv1a(id_str.as_bytes());
                let entry = physics.entry(wid).or_insert_with(|| ScrollPhysics {
                    pos_x: 0.0,
                    pos_y: 0.0,
                    vel_x: 0.0,
                    vel_y: 0.0,
                    is_dragging: false,
                    snap_target_x: None,
                    spring_start_x: 0.0,
                    spring_start_vel_x: 0.0,
                    spring_time: 0.0,
                    last_snap_page: 0,
                    snap_just_completed: false,
                    snap_x: None,
                    rubber_band: None,
                    page_count: None,
                    on_snap: None,
                });
                // Config güncelle & snap_x değiştiyse pos_x'i yeni genişliğe göre re-align et
                if let (Some(new_snap_x), Some(old_snap_x)) = (*snap_x, entry.snap_x) {
                    if (new_snap_x - old_snap_x).abs() > 0.5 && !entry.is_dragging {
                        entry.pos_x = (entry.last_snap_page as f32).max(0.0) * new_snap_x;
                        entry.snap_target_x = None;
                    }
                }
                entry.snap_x = *snap_x;
                entry.rubber_band = *rubber_band;
                entry.page_count = *page_count;
                entry.on_snap = on_snap.clone();
            }
        }
    }

    // Rekürsif tarama
    match element {
        crate::core::types::Element::Container { children, .. }
        | crate::core::types::Element::ScrollView { children, .. }
        | crate::core::types::Element::SharedView { children, .. } => {
            for child in children {
                sync_scroll_physics_from_tree(child, physics);
            }
        }
        _ => {}
    }
}

/// Hesaplanan fizik konumunu element tree'deki ScrollView'a yazar.
/// draw_ui bu de\u011feri kullanarak do\u011fru ofset ile \u00e7izer.
fn inject_physics_to_tree(
    element: &mut crate::core::types::Element,
    target_id: u64,
    pos_x: f32,
    pos_y: f32,
) {
    if let crate::core::types::Element::ScrollView { id, scroll_x, scroll_y, .. } = element {
        if let Some(id_str) = id {
            if crate::core::ui::widget::fnv1a(id_str.as_bytes()) == target_id {
                *scroll_x = pos_x;
                *scroll_y = pos_y;
                return;
            }
        }
    }
    match element {
        crate::core::types::Element::Container { children, .. }
        | crate::core::types::Element::ScrollView { children, .. }
        | crate::core::types::Element::SharedView { children, .. } => {
            for child in children {
                inject_physics_to_tree(child, target_id, pos_x, pos_y);
            }
        }
        _ => {}
    }
}

fn update_indicator_dots_in_element(
    element: &mut crate::core::types::Element,
    active_page: i32,
    vmin_px: f32,
) -> bool {
    if let crate::core::types::Element::Container { id, children, style } = element {
        if let Some(id_str) = id {
            if id_str == "page_indicator_container" {
                let is_landscape = style.flex_direction == crate::core::style::FlexDirection::Column;
                for (p, child_el) in children.iter_mut().enumerate() {
                    let is_active = (p as i32) == active_page;
                    if let crate::core::types::Element::Container { style: dot_style, .. } = child_el {
                        dot_style.background_color = Some(if is_active { 0xFF00_E5FF } else { 0x44FF_FFFF });
                        if is_landscape {
                            dot_style.height = crate::core::style::Dimension::Pixels(if is_active { 3.6 * vmin_px } else { 1.6 * vmin_px });
                            dot_style.width = crate::core::style::Dimension::Pixels(1.6 * vmin_px);
                        } else {
                            dot_style.width = crate::core::style::Dimension::Pixels(if is_active { 3.6 * vmin_px } else { 1.6 * vmin_px });
                            dot_style.height = crate::core::style::Dimension::Pixels(1.6 * vmin_px);
                        }
                    }
                }
                return true;
            }
        }
        for child in children {
            if update_indicator_dots_in_element(child, active_page, vmin_px) {
                return true;
            }
        }
    } else if let crate::core::types::Element::ScrollView { children, .. } = element {
        for child in children {
            if update_indicator_dots_in_element(child, active_page, vmin_px) {
                return true;
            }
        }
    }
    false
}

fn update_max_scroll_cache(
    state: &mut AppState,
    root_element: &crate::core::types::Element,
    layout_tree: &crate::core::layout::LayoutNode,
) {
    state.cached_max_scroll.clear();
    let mut search = vec![(root_element, layout_tree)];
    while let Some((el, lay)) = search.pop() {
        if let crate::core::types::Element::ScrollView { id, .. } = el {
            if let Some(id_str) = id {
                let sv_id = crate::core::ui::widget::fnv1a(id_str.as_bytes());
                let view_height = lay.rect.height;
                let mut min_y = f32::MAX;
                let mut max_y = f32::MIN;
                for child in &lay.children {
                    if child.rect.y < min_y { min_y = child.rect.y; }
                    if child.rect.y + child.rect.height > max_y { max_y = child.rect.y + child.rect.height; }
                }
                let max_scroll = if min_y <= max_y { (max_y - min_y - view_height).max(0.0) } else { 0.0 };
                state.cached_max_scroll.insert(sv_id, max_scroll);
            }
        }
        if let crate::core::types::Element::Container { children, .. } | crate::core::types::Element::ScrollView { children, .. } = el {
            for (child, child_lay) in children.iter().zip(lay.children.iter()) {
                search.push((child, child_lay));
            }
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

    let mut last_frame_time = std::time::Instant::now();

    while state.running {
        let now = std::time::Instant::now();
        let dt = now.duration_since(last_frame_time).as_secs_f32();
        last_frame_time = now;

        let current_ui_version = crate::core::types::UI_VERSION.load(std::sync::atomic::Ordering::Relaxed);
        let mut ui_changed = current_ui_version != state.last_ui_version;

        let physics_active = state
            .scroll_physics
            .values()
            .any(|p| p.is_dragging || p.vel_x.abs() > 0.5 || p.snap_target_x.is_some());
        let mut needs_redraw = !state.kinetic_scrolls.is_empty()
            || state.transition_manager.states.values().any(|s| s.is_active)
            || physics_active
            || ui_changed;

        let poll_timeout = if needs_redraw {
            Duration::ZERO
        } else {
            Duration::from_millis(16)
        };

        // ── Phase 1: collect ALL events first, render NOTHING inside ──────────
        // FIX (Sorun 1+2): poll_events callback'i aynı frame'de birden fazla
        // kez çağrılabilir. Render bloğunu buradan çıkararak her frame'de
        // yalnızca TEK bir render + swap_buffers garantiliyoruz.
        app.poll_events(Some(poll_timeout), |event| {
            match event {
                PollEvent::Wake | PollEvent::Timeout => {
                    if !state.kinetic_scrolls.is_empty()
                        || state.transition_manager.states.values().any(|s| s.is_active)
                    {
                        needs_redraw = true;
                    }
                }
                PollEvent::Main(main_event) => match main_event {
                    MainEvent::InitWindow { .. } => {
                        if egl_state.is_none() {
                            match EglContextState::new() {
                                Ok(s) => egl_state = Some(s),
                                Err(e) => dev_err!("{}: {e}", obfstr!("[EGL Error] Failed to create EGL state")),
                            }
                        }
                        if let Some(ref mut egl) = egl_state
                            && let Some(window) = app.native_window()
                        {
                            if let Err(e) = egl.bind_window(&window) {
                                dev_err!("{}: {e}", obfstr!("[EGL Error] Failed to bind window"));
                            } else {
                                dev_log!("{}", obfstr!("[EGL] Window successfully bound to EGL"));
                            }
                            let width = f32::from(u16::try_from(window.width()).unwrap_or(0));
                            let height = f32::from(u16::try_from(window.height()).unwrap_or(0));
                            state.event_bus.push(UiEvent::WindowResized(width, height));
                        }
                        // Recents return / Surface re-init: Invalidate layout and page stubs for fresh orientation render
                        state.cached_layout = None;
                        state.cached_max_scroll.clear();
                        state.virtual_page_manager.invalidate_on_resize(&mut root_element);
                        needs_redraw = true;
                    }
                    MainEvent::WindowResized { .. }
                    | MainEvent::ContentRectChanged { .. }
                    | MainEvent::RedrawNeeded { .. } => {
                        if let Some(ref mut egl) = egl_state
                            && let Some(window) = app.native_window()
                        {
                            let _ = egl.bind_window(&window);
                            let width = f32::from(u16::try_from(window.width()).unwrap_or(0));
                            let height = f32::from(u16::try_from(window.height()).unwrap_or(0));
                            state.event_bus.push(UiEvent::WindowResized(width, height));
                        }
                        state.cached_layout = None;
                        state.cached_max_scroll.clear();
                        state.virtual_page_manager.invalidate_on_resize(&mut root_element);
                        needs_redraw = true;
                    }
                    MainEvent::InputAvailable => {
                        if let Ok(mut iter) = app.input_events_iter() {
                            loop {
                                let had_event = iter.next(|input_event| {
                                    // Reuse cached layout from last frame — avoids redundant calculate_layout per touch event
                                    let layout_clone = state.cached_layout.clone();
                                    if let Some(ref cached) = layout_clone {
                                        super::input::handle_input_event(
                                            input_event,
                                            &mut state,
                                            &root_element,
                                            cached,
                                        )
                                    } else {
                                        InputStatus::Unhandled
                                    }
                                });

                                if !had_event {
                                    break;
                                }
                            }
                        }
                        needs_redraw = true;
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
            }
        });

        // ── Phase 2: render ONCE per frame, after all events are collected ─────
        if needs_redraw
            && let Some(ref mut egl) = egl_state
            && let Some(ref mut renderer) = egl.renderer
            && let Some(window) = app.native_window()
        {
            // tick() her frame çalışmalı (JS timer/animation callback'leri için)
            state.plugin_registry.tick();

            let width =
                f32::from(u16::try_from(window.width()).expect("window width fits in u16"));
            let height =
                f32::from(u16::try_from(window.height()).expect("window height fits in u16"));

            let screen_changed = (width - state.cached_screen_size.0).abs() > 0.5
                || (height - state.cached_screen_size.1).abs() > 0.5;

            if screen_changed {
                state.cached_screen_size = (width, height);
                state.cached_safe_area = crate::platform::android::jni::get_safe_area(&app)
                    .map(|(top, bottom)| {
                        (
                            f32::from(i16::try_from(top).expect("safe-area top fits in i16")),
                            f32::from(
                                i16::try_from(bottom).expect("safe-area bottom fits in i16"),
                            ),
                        )
                    })
                    .unwrap_or((0.0, 0.0));
                state.cached_density = crate::platform::android::jni::get_density();

                state.cached_layout = None;
                state.cached_max_scroll.clear();

                let content_h = height - state.cached_safe_area.0 - state.cached_safe_area.1;
                crate::core::types::SCREEN_WIDTH
                    .store(width.to_bits(), std::sync::atomic::Ordering::Relaxed);
                crate::core::types::SCREEN_HEIGHT
                    .store(content_h.to_bits(), std::sync::atomic::Ordering::Relaxed);

                ui_changed = true;
            } else {
                let content_h = height - state.cached_safe_area.0 - state.cached_safe_area.1;
                crate::core::types::SCREEN_WIDTH
                    .store(width.to_bits(), std::sync::atomic::Ordering::Relaxed);
                crate::core::types::SCREEN_HEIGHT
                    .store(content_h.to_bits(), std::sync::atomic::Ordering::Relaxed);
            }

            let is_first_frame = matches!(&root_element, crate::core::types::Element::Container { children, .. } if children.is_empty());
            if ui_changed || is_first_frame {
                if let Some(new_root) = state.plugin_registry.build_ui() {
                    root_element = new_root;
                }
            }

            if screen_changed || is_first_frame {
                let content_h = height - state.cached_safe_area.0 - state.cached_safe_area.1;
                state.virtual_page_manager.on_viewport_resized(width, content_h, &mut root_element);

                if let Some((pixels, w, h)) = crate::platform::android::jni::bridge::get_system_wallpaper_pixels(width as u32, height as u32) {
                    renderer.load_wallpaper(&pixels, w, h);
                }
            }

            // Faz 5: Strip off-screen pages before Taffy layout calculation
            state.virtual_page_manager.virtualize_tree(&mut root_element);

            // Sync ScrollPhysics config from element tree (yeni snap_x, page_count vs. al)
            sync_scroll_physics_from_tree(&root_element, &mut state.scroll_physics);

            state.transition_manager.sync_tree(&root_element);
            let _ = state.transition_manager.tick(dt);

            let (safe_area_top, safe_area_bottom) = state.cached_safe_area;
            let (density, scaled_density) = state.cached_density;

            let metrics = ScreenMetrics::from_scale(
                width,
                height - safe_area_top - safe_area_bottom,
                density,
                scaled_density,
            );

            // Orientation Match Check: If cached layout aspect ratio (landscape vs portrait) mismatches current window, invalidate layout!
            if let Some(ref cached) = state.cached_layout {
                let cached_is_landscape = cached.rect.width > cached.rect.height;
                let window_is_landscape = width > height;
                if cached_is_landscape != window_is_landscape {
                    state.cached_layout = None;
                    state.virtual_page_manager.invalidate_on_resize(&mut root_element);
                }
            }

            let layout_tree = if let Some(ref cached) = state.cached_layout
                && !screen_changed
                && !ui_changed
                && (physics_active || state.active_scrollview_drag.is_some())
            {
                cached.clone()
            } else {
                state.last_ui_version = current_ui_version;
                let fresh = Arc::new(calculate_layout(
                    &root_element,
                    Size::new(width, height - safe_area_top - safe_area_bottom),
                    0.0,
                    safe_area_top,
                ));
                update_max_scroll_cache(&mut state, &root_element, &fresh);
                state.cached_layout = Some(fresh.clone());
                fresh
            };

            let get_max_scroll = |target_id: Option<u64>| -> f32 {
                target_id.and_then(|id| state.cached_max_scroll.get(&id).copied()).unwrap_or(0.0)
            };

            state.kinetic_scrolls.retain_mut(|k| {
                // Stop only when velocity is sub-pixel (< 0.5 px/frame)
                if k.velocity_x.abs() > 0.5 || k.velocity_y.abs() > 0.5 {
                    state.event_bus.push(UiEvent::Scroll(
                        Some(k.sv_id),
                        k.velocity_x,
                        k.velocity_y,
                        999_999.0,
                        get_max_scroll(Some(k.sv_id)),
                    ));
                    // Friction: 0.88 → smooth long-coast like native Android
                    k.velocity_x *= 0.88;
                    k.velocity_y *= 0.88;
                    true
                } else {
                    false
                }
            });

            // ── Rust-owned ScrollPhysics tick ──────────────────────────────────
            let content_h_for_js = height - safe_area_top - safe_area_bottom;
            let vmin_px = width.min(content_h_for_js) / 100.0;
            let phys_ids: Vec<u64> = state.scroll_physics.keys().copied().collect();
            for sv_id in phys_ids {
                if let Some(phys) = state.scroll_physics.get_mut(&sv_id) {
                    if phys.tick(dt) {
                        #[allow(unused_assignments)]
                        { needs_redraw = true; }
                    }

                    if let Some(snap_width) = phys.snap_x {
                        if snap_width > 0.0 {
                            let max_page = (phys.page_count.unwrap_or(1) as i32 - 1).max(0);
                            let max_x = snap_width * (max_page as f32);

                            // 1. Strict Page Bounds Clamping: Prevent scroll_x offset explosion / black screen
                            phys.pos_x = phys.pos_x.clamp(0.0, max_x);

                            let current_page = (phys.pos_x / snap_width).round() as i32;
                            let clamped_page = current_page.clamp(0, max_page) as usize;

                            let page_window_changed = state.virtual_page_manager.update_predicted_page(
                                clamped_page,
                                &mut root_element,
                                phys.vel_x,
                            );

                            if page_window_changed {
                                state.cached_layout = None; // Pre-materialized sayfa için Taffy re-layout tetikle
                                #[allow(unused_assignments)]
                                { needs_redraw = true; }
                            }

                            phys.last_snap_page = clamped_page as i32;

                            if phys.snap_just_completed && phys.on_snap.is_some() {
                                state.event_bus.push(UiEvent::PageSnapped {
                                    widget_id: sv_id,
                                    page: clamped_page as i32,
                                });
                            }
                        }
                    }

                    // Physics konumunu element tree'ye geri yaz (draw_ui kullanacak)
                    let pos_x = phys.pos_x;
                    let pos_y = phys.pos_y;
                    inject_physics_to_tree(&mut root_element, sv_id, pos_x, pos_y);

                    // 3. Indicator Sync: Güncel active_page ile gösterge noktalarını anında senkronize et
                    update_indicator_dots_in_element(&mut root_element, phys.last_snap_page, vmin_px);
                }
            }


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
                            if let Some(pkg_name) = src.strip_prefix(obfstr!("app-icon://")) {
                                if let Some((pixels, w, h)) = crate::platform::android::jni::bridge::get_app_icon_pixels(pkg_name) {
                                    renderer.load_image(&id, &pixels, w, h);
                                    dev_log!("{} {}", obfstr!("Successfully loaded app icon for"), pkg_name);
                                } else {
                                    dev_err!("{} {}", obfstr!("Failed to load app icon for"), pkg_name);
                                }
                            } else {
                                let asset_path = src.replace(obfstr!(".plugins/"), "").replace('\\', "/");

                                if let Ok(cstr) = std::ffi::CString::new(asset_path.clone()) {
                                    if let Some(mut asset) = app.asset_manager().open(cstr.as_c_str()) {
                                        use std::io::Read;
                                        let mut buffer = Vec::new();
                                        if asset.read_to_end(&mut buffer).is_ok() {
                                            if let Ok(img) = image::load_from_memory(&buffer) {
                                                let rgba = img.to_rgba8();
                                                let (w, h) = rgba.dimensions();
                                                renderer.load_image(&id, rgba.as_raw(), w, h);
                                                dev_log!("{} {} {}", obfstr!("Successfully loaded image"), asset_path, obfstr!("from Android assets"));
                                            } else {
                                                dev_err!("{} {}", obfstr!("Failed to parse image data for"), asset_path);
                                            }
                                        }
                                    } else {
                                        dev_err!("{} {}", obfstr!("Failed to open image asset:"), asset_path);
                                    }
                                }
                            }
                        }
                        crate::core::Action::FocusTextInput(_id) => {
                            app.show_soft_input(true);
                        }
                        crate::core::Action::BlurTextInput => {
                            app.hide_soft_input(true);
                        }
                        _ => {
                            let _ =
                                crate::platform::android::jni::intent::launch_action(action);
                        }
                    }
                }
            }

            renderer.begin_frame(width, height);
            renderer.clear(BACKGROUND);
            renderer.draw_wallpaper(width, height);

            draw_ui(
                renderer,
                &root_element,
                &layout_tree,
                &metrics,
                &state.style_map,
                &state.data_map,
                &state.transition_manager,
                1.0,
                0.0,
                0.0,
            );

            renderer.end_frame();
            egl.swap_buffers();
        }
    }
}
