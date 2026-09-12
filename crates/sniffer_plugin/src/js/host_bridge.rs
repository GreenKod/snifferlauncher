use sniffer_core::types::AppInfo;
use std::sync::RwLock;

pub trait HostPlatformBridge: Send + Sync {
    fn get_application_list(&self, _caller_id: &str) -> Result<Vec<AppInfo>, String> {
        Ok(Vec::new())
    }
    fn launch_app(&self, _caller_id: &str, _package_name: &str) -> Result<(), String> {
        Ok(())
    }
    fn open_default_home_picker(&self, _caller_id: &str) -> Result<(), String> {
        Ok(())
    }
    fn request_permissions(
        &self,
        _caller_id: &str,
        _perms: &[String],
    ) -> Result<Vec<String>, String> {
        Ok(Vec::new())
    }
}

static HOST_BRIDGE: RwLock<Option<Box<dyn HostPlatformBridge>>> = RwLock::new(None);

pub fn set_host_bridge(bridge: Box<dyn HostPlatformBridge>) {
    if let Ok(mut lock) = HOST_BRIDGE.write() {
        *lock = Some(bridge);
    }
}

pub fn get_application_list(caller_id: &str) -> Result<Vec<AppInfo>, String> {
    if let Ok(lock) = HOST_BRIDGE.read() {
        if let Some(bridge) = lock.as_ref() {
            return bridge.get_application_list(caller_id);
        }
    }
    Ok(Vec::new())
}

pub fn launch_app(caller_id: &str, package_name: &str) -> Result<(), String> {
    if let Ok(lock) = HOST_BRIDGE.read() {
        if let Some(bridge) = lock.as_ref() {
            return bridge.launch_app(caller_id, package_name);
        }
    }
    Ok(())
}

pub fn open_default_home_picker(caller_id: &str) -> Result<(), String> {
    if let Ok(lock) = HOST_BRIDGE.read() {
        if let Some(bridge) = lock.as_ref() {
            return bridge.open_default_home_picker(caller_id);
        }
    }
    Ok(())
}

pub fn request_permissions(caller_id: &str, perms: &[String]) -> Result<Vec<String>, String> {
    if let Ok(lock) = HOST_BRIDGE.read() {
        if let Some(bridge) = lock.as_ref() {
            return bridge.request_permissions(caller_id, perms);
        }
    }
    Ok(Vec::new())
}
