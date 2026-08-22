use super::physics_sync::ScrollPhysics;
use android_activity::AndroidApp;
use sniffer_core::ui::data_map::DataMap;
use sniffer_core::ui::event::EventBus;
use sniffer_core::ui::style_map::StyleMap;
use sniffer_core::{Action, Point};
use sniffer_pkg::{MemoryTrimLevel, PackageRegistry};
use sniffer_plugin::registry::PluginRegistry;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, RwLock};

pub struct KineticScroll {
    pub sv_id: u64,
    pub velocity_x: f32,
    pub velocity_y: f32,
}

pub struct AppState {
    pub running: bool,
    pub touch_start_pos: Point,
    pub last_touch_pos: Point,
    pub hovered_btn: Option<u64>,
    pub active_scrollview_drag: Option<u64>,
    pub last_drag_delta: (f32, f32),
    pub drag_history: VecDeque<(f32, f32, std::time::Instant)>,
    pub kinetic_scrolls: Vec<KineticScroll>,
    pub scroll_physics: std::collections::HashMap<u64, ScrollPhysics>,

    pub event_bus: EventBus,
    pub style_map: StyleMap,
    pub data_map: DataMap,
    pub action_queue: Arc<Mutex<Vec<Action>>>,
    pub plugin_registry: PluginRegistry,
    pub pkg_registry: Arc<RwLock<PackageRegistry>>,
    pub transition_manager: sniffer_core::anim::TransitionManager,

    pub cached_safe_area: (f32, f32),
    pub cached_density: (f32, f32),
    pub cached_screen_size: (f32, f32),
    pub cached_layout: Option<Arc<sniffer_core::layout::LayoutNode>>,
    pub cached_max_scroll: std::collections::HashMap<u64, f32>,
    pub last_ui_version: u64,
    pub layout_dirty: bool,
    pub memory_pressure_pending: bool,
    pub virtual_page_manager: sniffer_core::virtualization::VirtualPageManager,
    pub total_touch_drag_distance: f32,
    pub profiler: Arc<Mutex<sniffer_core::profiler::FrameProfiler>>,
}

impl AppState {
    #[must_use]
    pub fn new(app: &AndroidApp) -> Self {
        let mut plugin_registry = PluginRegistry::default();
        let action_queue = Arc::new(Mutex::new(Vec::new()));

        plugin_registry
            .vault()
            .update_system_theme(sniffer_core::vault::SystemTheme::default());

        if let Ok(apps) = crate::jni::get_application_list() {
            if !apps.is_empty() {
                plugin_registry.vault().update_system_apps(apps);
            }
        }

        let mut pkg_reg = PackageRegistry::new(plugin_registry.vault());
        pkg_reg.register_service(Arc::new(
            sniffer_pkg::perf_monitor::PerfMonitorPackage::new(),
        ));
        pkg_reg.register_widget(Arc::new(sniffer_pkg::scroll_view::ScrollViewPackage::new()));

        let pkg_registry = Arc::new(RwLock::new(pkg_reg));
        plugin_registry.set_pkg_registry(Arc::clone(&pkg_registry));

        sniffer_plugin::PluginLoader::register_all_from_assets(
            &mut plugin_registry,
            &app.asset_manager(),
            &action_queue,
        );

        Self {
            running: true,
            touch_start_pos: Point::zero(),
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
            pkg_registry,
            transition_manager: sniffer_core::anim::TransitionManager::default(),

            cached_safe_area: (0.0, 0.0),
            cached_density: (1.0, 1.0),
            cached_screen_size: (0.0, 0.0),
            cached_layout: None,
            cached_max_scroll: std::collections::HashMap::new(),
            last_ui_version: 0,
            layout_dirty: true,
            memory_pressure_pending: false,
            virtual_page_manager: sniffer_core::virtualization::VirtualPageManager::new(),
            total_touch_drag_distance: 0.0,
            profiler: Arc::new(Mutex::new(sniffer_core::profiler::FrameProfiler::default())),
        }
    }

    /// Handles system `onTrimMemory` / `onLowMemory` pressure events by instantly evicting LRU textures from GPU.
    pub fn trim_memory(&mut self, renderer: &mut sniffer_render::glow::GlowRenderer) {
        renderer.trim_memory();
        if let Ok(reg) = self.pkg_registry.read() {
            reg.trim_memory(MemoryTrimLevel::Critical);
        }
        self.cached_layout = None;
        self.layout_dirty = true;
        self.memory_pressure_pending = false;
    }
}

pub(crate) fn update_max_scroll_cache(
    state: &mut AppState,
    root_element: &sniffer_core::types::Element,
    layout_tree: &sniffer_core::layout::LayoutNode,
) {
    state.cached_max_scroll.clear();
    let mut search = vec![(root_element, layout_tree)];
    while let Some((el, lay)) = search.pop() {
        if let sniffer_core::types::Element::ScrollView {
            id: Some(id_str), ..
        } = el
        {
            let sv_id = sniffer_core::ui::widget::fnv1a(id_str.as_bytes());
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
            let max_scroll = if min_y <= max_y {
                (max_y - min_y - view_height).max(0.0)
            } else {
                0.0
            };
            state.cached_max_scroll.insert(sv_id, max_scroll);
        }
        if let sniffer_core::types::Element::Container { children, .. }
        | sniffer_core::types::Element::ScrollView { children, .. } = el
        {
            for (child, child_lay) in children.iter().zip(lay.children.iter()) {
                search.push((child, child_lay));
            }
        }
    }
}
