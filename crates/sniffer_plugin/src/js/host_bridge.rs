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
    /// Reactively wakes up the platform event loop from idle / sleep.
    fn wake(&self) {}
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

/// Dispatches a reactive wake request to the platform host bridge.
pub fn wake() {
    if let Ok(lock) = HOST_BRIDGE.read() {
        if let Some(bridge) = lock.as_ref() {
            bridge.wake();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct TestHostBridge {
        wake_count: Arc<AtomicUsize>,
    }

    impl HostPlatformBridge for TestHostBridge {
        fn wake(&self) {
            self.wake_count.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn test_host_bridge_wake_dispatch() {
        let wake_count = Arc::new(AtomicUsize::new(0));
        set_host_bridge(Box::new(TestHostBridge {
            wake_count: Arc::clone(&wake_count),
        }));

        assert_eq!(wake_count.load(Ordering::SeqCst), 0);
        wake();
        assert_eq!(wake_count.load(Ordering::SeqCst), 1);
        wake();
        assert_eq!(wake_count.load(Ordering::SeqCst), 2);
    }
}
