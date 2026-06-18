#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "windows")]
pub mod windows;

pub trait PermissionManager {
    fn has_microphone_permission(&self) -> bool;
    fn request_microphone_permission(&self);
    fn has_accessibility_permission(&self) -> bool;
    fn request_accessibility_permission(&self);
}

#[cfg(target_os = "macos")]
pub fn get_manager() -> Box<dyn PermissionManager> {
    Box::new(macos::MacOsPermissionManager {})
}

#[cfg(target_os = "windows")]
pub fn get_manager() -> Box<dyn PermissionManager> {
    Box::new(windows::WindowsPermissionManager {})
}
