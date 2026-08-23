use sniffer_core::dev_log;
use sniffer_core::scroll_physics::ScrollPhysics;
use sniffer_core::ui::data_map::DataMap;
use sniffer_core::ui::event::EventBus;
use sniffer_core::ui::style_map::StyleMap;
use sniffer_core::{Action, Point};
use sniffer_pkg::PackageRegistry;
use sniffer_plugin::registry::PluginRegistry;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, RwLock};

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
    pub mouse_down_pos: Option<Point>,
    pub total_drag_dist: f32,
    pub hovered_btn: Option<u64>,
    pub active_scrollview_drag: Option<u64>,
    pub last_drag_delta: (f32, f32),
    pub kinetic_scrolls: Vec<KineticScroll>,
    pub scroll_physics: HashMap<u64, ScrollPhysics>,
    pub drag_history: VecDeque<(f32, f32, std::time::Instant)>,

    pub event_bus: EventBus,
    pub style_map: StyleMap,
    pub data_map: DataMap,
    pub action_queue: Arc<Mutex<Vec<Action>>>,
    pub plugin_registry: PluginRegistry,
    pub pkg_registry: Arc<RwLock<PackageRegistry>>,
    pub transition_manager: sniffer_core::anim::TransitionManager,
    #[cfg(feature = "devkit")]
    pub profiler: Arc<Mutex<sniffer_core::profiler::FrameProfiler>>,
}

#[must_use]
pub fn detect_desktop_theme() -> sniffer_core::vault::SystemTheme {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("reg")
            .args([
                "query",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
                "/v",
                "AppsUseLightTheme",
            ])
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
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|dir| dir.join(".plugins"))),
            std::env::current_exe().ok().and_then(|p| {
                p.parent()
                    .and_then(|d| d.parent())
                    .map(|dir| dir.join(".plugins"))
            }),
            option_env!("CARGO_MANIFEST_DIR").map(|dir| std::path::Path::new(dir).join(".plugins")),
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

        #[cfg(feature = "devkit")]
        let profiler = Arc::new(Mutex::new(sniffer_core::profiler::FrameProfiler::new(120)));

        let perf_pkg = Arc::new(pkg_perfmon::PerfMonitorPackage::new());
        #[cfg(feature = "devkit")]
        perf_pkg.set_profiler(Arc::clone(&profiler));

        let mut pkg_reg = PackageRegistry::new(plugin_registry.vault());
        pkg_reg.register_service(perf_pkg);
        pkg_reg.register_widget(Arc::new(pkg_scroll::ScrollViewPackage::new()));

        let pkg_registry = Arc::new(RwLock::new(pkg_reg));
        plugin_registry.set_pkg_registry(Arc::clone(&pkg_registry));

        sniffer_plugin::PluginLoader::new(&plugins_dir)
            .register_all(&mut plugin_registry, &action_queue);

        Self {
            running: true,
            window_focused: true,
            is_mouse_down: false,
            focused_input_id: None,
            last_mouse_pos: Point::new(-9999.0, -9999.0),
            mouse_down_pos: None,
            total_drag_dist: 0.0,
            hovered_btn: None,
            active_scrollview_drag: None,
            last_drag_delta: (0.0, 0.0),
            kinetic_scrolls: Vec::new(),
            scroll_physics: HashMap::new(),
            drag_history: VecDeque::new(),

            event_bus: EventBus::default(),
            style_map: StyleMap::default(),
            data_map: DataMap::default(),
            action_queue,
            plugin_registry,
            pkg_registry,
            transition_manager: sniffer_core::anim::TransitionManager::default(),
            #[cfg(feature = "devkit")]
            profiler,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
