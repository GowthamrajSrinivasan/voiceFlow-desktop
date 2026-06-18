use crate::permissions::PermissionManager;

pub struct WindowsPermissionManager {}

impl PermissionManager for WindowsPermissionManager {
    fn has_microphone_permission(&self) -> bool {
        // Mock check for now
        true
    }

    fn request_microphone_permission(&self) {
        // Implementation for windows
    }

    fn has_accessibility_permission(&self) -> bool {
        // Windows doesn't typically require accessibility permission for global hotkeys or Enigo text injection
        true
    }

    fn request_accessibility_permission(&self) {
        // No-op
    }
}
