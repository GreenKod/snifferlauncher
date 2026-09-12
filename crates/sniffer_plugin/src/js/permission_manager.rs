use crate::dev_err;
use obfstr::obfstr;
use std::sync::Mutex;

// Standard Launcher & UI permissions
pub const PERM_UI: &str = "plugin.permission.UI";
pub const PERM_IPC: &str = "plugin.permission.IPC";
pub const PERM_IMAGE: &str = "plugin.permission.IMAGE";
pub const PERM_INPUT: &str = "plugin.permission.INPUT";
pub const PERM_SHARED_VIEW: &str = "shared_view.provider";
pub const PERM_PLUGIN_APP_LAUNCH: &str = "plugin.permission.APP_LAUNCH";

// Android Platform permissions
pub const PERM_ANDROID_QUERY_PACKAGES: &str = "android.permission.QUERY_ALL_PACKAGES";
pub const PERM_ANDROID_READ_STORAGE: &str = "android.permission.READ_EXTERNAL_STORAGE";
pub const PERM_ANDROID_READ_MEDIA_IMAGES: &str = "android.permission.READ_MEDIA_IMAGES";
pub const PERM_ANDROID_SET_WALLPAPER: &str = "android.permission.SET_WALLPAPER";
pub const PERM_LAUNCH_APP: &str = "android.permission.LAUNCH_APP";

/// Returns true if the given permission is a safe baseline capability
/// that can be granted automatically to sub-plugins and widgets.
#[must_use]
pub fn is_baseline_permission(perm: &str) -> bool {
    matches!(perm, PERM_UI | PERM_IPC | PERM_SHARED_VIEW)
}

/// Checks whether the plugin is authorized to launch external applications.
pub fn has_launch_permission(
    plugin_permissions: &[String],
    granted_permissions: &Mutex<Vec<String>>,
) -> bool {
    permission_granted(PERM_LAUNCH_APP, plugin_permissions, granted_permissions)
        || permission_granted(
            PERM_PLUGIN_APP_LAUNCH,
            plugin_permissions,
            granted_permissions,
        )
}

/// Check if the plugin declared and was granted the requested permission.
pub fn permission_granted(
    permission: &str,
    plugin_permissions: &[String],
    granted_permissions: &Mutex<Vec<String>>,
) -> bool {
    let permission_string = permission.to_string();

    if !plugin_permissions.contains(&permission_string) {
        dev_err!(
            "{} '{}' {}.",
            obfstr!("Security Audit: Plugin is not authorized for"),
            permission,
            obfstr!("because it is not declared in manifest permissions (DENIED)")
        );
        return false;
    }

    match granted_permissions.lock() {
        Ok(lock) => {
            if lock.contains(&permission_string) {
                true
            } else {
                dev_err!(
                    "{} '{}'. {} requestPermissions([{}]) {}.",
                    obfstr!("Security Audit: Plugin does not have granted permission"),
                    permission,
                    obfstr!("Call"),
                    permission,
                    obfstr!("first (DENIED)")
                );
                false
            }
        }
        Err(_) => {
            dev_err!(
                "{} '{}'.",
                obfstr!("Security Audit: Failed to acquire granted permissions lock for"),
                permission
            );
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PERM_IMAGE, PERM_INPUT, PERM_IPC, PERM_UI, is_baseline_permission, permission_granted,
    };
    use std::sync::Mutex;

    #[test]
    fn test_is_baseline_permission() {
        assert!(is_baseline_permission(PERM_UI));
        assert!(is_baseline_permission(PERM_IPC));
        assert!(is_baseline_permission("shared_view.provider"));
        assert!(!is_baseline_permission(PERM_INPUT));
        assert!(!is_baseline_permission(PERM_IMAGE));
        assert!(!is_baseline_permission(
            "android.permission.QUERY_ALL_PACKAGES"
        ));
    }

    #[test]
    fn permission_granted_returns_false_for_undeclared_permission() {
        let plugin_permissions = vec!["plugin.permission.UI".to_string()];
        let granted_permissions = Mutex::new(vec!["plugin.permission.UI".to_string()]);

        assert!(!permission_granted(
            "plugin.permission.IPC",
            &plugin_permissions,
            &granted_permissions,
        ));
    }

    #[test]
    fn permission_granted_returns_true_for_granted_permission() {
        let plugin_permissions = vec!["plugin.permission.UI".to_string()];
        let granted_permissions = Mutex::new(vec!["plugin.permission.UI".to_string()]);

        assert!(permission_granted(
            "plugin.permission.UI",
            &plugin_permissions,
            &granted_permissions,
        ));
    }

    #[test]
    fn test_has_launch_permission_gating() {
        use super::{PERM_LAUNCH_APP, PERM_PLUGIN_APP_LAUNCH, has_launch_permission};

        // Case 1: Neither declared nor granted
        let empty_declared = vec![];
        let empty_granted = Mutex::new(vec![]);
        assert!(!has_launch_permission(&empty_declared, &empty_granted));

        // Case 2: Declared android.permission.LAUNCH_APP and granted
        let android_declared = vec![PERM_LAUNCH_APP.to_string()];
        let android_granted = Mutex::new(vec![PERM_LAUNCH_APP.to_string()]);
        assert!(has_launch_permission(&android_declared, &android_granted));

        // Case 3: Declared plugin.permission.APP_LAUNCH and granted
        let plugin_declared = vec![PERM_PLUGIN_APP_LAUNCH.to_string()];
        let plugin_granted = Mutex::new(vec![PERM_PLUGIN_APP_LAUNCH.to_string()]);
        assert!(has_launch_permission(&plugin_declared, &plugin_granted));

        // Case 4: Declared but NOT granted (e.g. pending user consent)
        let declared_only = vec![PERM_LAUNCH_APP.to_string()];
        let not_granted = Mutex::new(vec![]);
        assert!(!has_launch_permission(&declared_only, &not_granted));
    }
}
