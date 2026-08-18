use sniffer_core::types::AppInfo;
use std::sync::RwLock;

pub trait HostPlatformBridge: Send + Sync {
    fn get_application_list(&self) -> Result<Vec<AppInfo>, String> {
        Ok(Vec::new())
    }
    fn launch_app(&self, _package_name: &str) -> Result<(), String> {
        Ok(())
    }
    fn open_default_home_picker(&self) -> Result<(), String> {
        Ok(())
    }
    fn request_permissions(&self, _perms: &[String]) {}
}

static HOST_BRIDGE: RwLock<Option<Box<dyn HostPlatformBridge>>> = RwLock::new(None);

pub fn set_host_bridge(bridge: Box<dyn HostPlatformBridge>) {
    if let Ok(mut lock) = HOST_BRIDGE.write() {
        *lock = Some(bridge);
    }
}

pub fn get_application_list() -> Result<Vec<AppInfo>, String> {
    if let Ok(lock) = HOST_BRIDGE.read() {
        if let Some(bridge) = lock.as_ref() {
            return bridge.get_application_list();
        }
    }
    Ok(Vec::new())
}

pub fn launch_app(package_name: &str) -> Result<(), String> {
    if let Ok(lock) = HOST_BRIDGE.read() {
        if let Some(bridge) = lock.as_ref() {
            return bridge.launch_app(package_name);
        }
    }
    Ok(())
}

pub fn open_default_home_picker() -> Result<(), String> {
    if let Ok(lock) = HOST_BRIDGE.read() {
        if let Some(bridge) = lock.as_ref() {
            return bridge.open_default_home_picker();
        }
    }
    Ok(())
}

pub fn request_permissions(perms: &[String]) {
    if let Ok(lock) = HOST_BRIDGE.read() {
        if let Some(bridge) = lock.as_ref() {
            bridge.request_permissions(perms);
        }
    }
}
