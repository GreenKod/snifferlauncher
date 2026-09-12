use sniffer_core::ScreenMetrics;
use std::collections::HashMap;
use std::sync::Arc;

pub enum RenderMessage {
    InitWindow(usize, f32, f32),
    WindowResized(usize, f32, f32),
    TerminateWindow,
    LowMemory,
    Destroy,
}

#[derive(Clone)]
pub struct SharedRenderState {
    pub root_element: sniffer_core::types::Element,
    pub layout_tree: Arc<sniffer_core::layout::LayoutNode>,
    pub metrics: ScreenMetrics,
    pub style_map: sniffer_core::ui::style_map::StyleMap,
    pub data_map: sniffer_core::ui::data_map::DataMap,
    pub transition_manager: sniffer_core::anim::TransitionManager,
    pub shaders_warmed_up: bool,
}

pub struct AndroidHostBridge;

impl sniffer_plugin::js::host_bridge::HostPlatformBridge for AndroidHostBridge {
    fn get_application_list(
        &self,
        caller_id: &str,
    ) -> Result<Vec<sniffer_core::types::AppInfo>, String> {
        sniffer_core::dev_log!(
            "AndroidHostBridge: get_application_list requested by '{caller_id}'"
        );
        crate::jni::bridge::get_application_list()
    }
    fn launch_app(&self, caller_id: &str, package_name: &str) -> Result<(), String> {
        sniffer_core::dev_log!(
            "AndroidHostBridge: launch_app '{package_name}' requested by '{caller_id}'"
        );
        crate::jni::intent::launch_app(package_name)
    }
    fn open_default_home_picker(&self, caller_id: &str) -> Result<(), String> {
        sniffer_core::dev_log!(
            "AndroidHostBridge: open_default_home_picker requested by '{caller_id}'"
        );
        crate::jni::bridge::open_default_home_picker();
        Ok(())
    }
    fn request_permissions(
        &self,
        caller_id: &str,
        perms: &[String],
    ) -> Result<Vec<String>, String> {
        sniffer_core::dev_log!("AndroidHostBridge: request_permissions {perms:?} by '{caller_id}'");
        let _ = crate::jni::bridge::request_permissions(perms);
        Ok(perms.to_vec())
    }
}

#[allow(clippy::implicit_hasher)]
pub fn collect_layout_rects<'a>(
    node: &'a sniffer_core::layout::LayoutNode,
    map: &mut HashMap<&'a str, sniffer_core::math::Rect>,
) {
    if let Some(id) = node.element.id() {
        map.insert(id, node.rect);
    }
    for child in &node.children {
        collect_layout_rects(child, map);
    }
}
