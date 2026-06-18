use crate::permissions::PermissionManager;
use std::process::Command;

pub struct MacOsPermissionManager {}

impl PermissionManager for MacOsPermissionManager {
    fn has_microphone_permission(&self) -> bool {
        // Mock check for now. In a real app, use AVFoundation
        // AVCaptureDevice::authorizationStatusForMediaType
        true
    }

    fn request_microphone_permission(&self) {
        // In a real app, request via AVFoundation
    }

    fn has_accessibility_permission(&self) -> bool {
        let output = Command::new("osascript")
            .arg("-e")
            .arg("tell application \"System Events\" to get UI elements enabled")
            .output();

        if let Ok(output) = output {
            let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
            return result == "true";
        }
        false
    }

    fn request_accessibility_permission(&self) {
        // Prompt user to open System Preferences if needed
        let _ = Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn();
    }
}
