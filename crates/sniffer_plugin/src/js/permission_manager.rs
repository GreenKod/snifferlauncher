use crate::{dev_err};
use obfstr::obfstr;
use std::sync::Mutex;

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
            obfstr!("Plugin is not authorized for"),
            permission,
            obfstr!("because it is not declared in manifest permissions")
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
                    obfstr!("Plugin does not have granted permission"),
                    permission,
                    obfstr!("Call"),
                    permission,
                    obfstr!("first")
                );
                false
            }
        }
        Err(_) => {
            dev_err!(
                "{} '{}'.",
                obfstr!("Failed to check granted permissions for"),
                permission
            );
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::permission_granted;
    use std::sync::Mutex;

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

