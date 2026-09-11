use crate::dev_err;
use obfstr::obfstr;
use std::sync::Mutex;

// Standard Launcher & UI permissions
pub const PERM_UI: &str = "plugin.permission.UI";
pub const PERM_IPC: &str = "plugin.permission.IPC";
pub const PERM_IMAGE: &str = "plugin.permission.IMAGE";
pub const PERM_INPUT: &str = "plugin.permission.INPUT";
pub const PERM_SHARED_VIEW: &str = "shared_view.provider";

// Android Platform permissions
pub const PERM_ANDROID_QUERY_PACKAGES: &str = "android.permission.QUERY_ALL_PACKAGES";
pub const PERM_ANDROID_READ_STORAGE: &str = "android.permission.READ_EXTERNAL_STORAGE";
pub const PERM_ANDROID_READ_MEDIA_IMAGES: &str = "android.permission.READ_MEDIA_IMAGES";
pub const PERM_ANDROID_SET_WALLPAPER: &str = "android.permission.SET_WALLPAPER";

/// Returns true if the given permission is a safe baseline capability
/// that can be granted automatically to sub-plugins and widgets.
#[must_use]
pub fn is_baseline_permission(perm: &str) -> bool {
    matches!(perm, PERM_UI | PERM_IPC | PERM_SHARED_VIEW)
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
    use super::{is_baseline_permission, permission_granted, PERM_IMAGE, PERM_INPUT, PERM_IPC, PERM_UI};
    use std::sync::Mutex;

    #[test]
    fn test_is_baseline_permission() {
        assert!(is_baseline_permission(PERM_UI));
        assert!(is_baseline_permission(PERM_IPC));
        assert!(is_baseline_permission("shared_view.provider"));
        assert!(!is_baseline_permission(PERM_INPUT));
        assert!(!is_baseline_permission(PERM_IMAGE));
        assert!(!is_baseline_permission("android.permission.QUERY_ALL_PACKAGES"));
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
}
