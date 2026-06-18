use crate::TextInjector;
use enigo::{Enigo, Keyboard, Settings};

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> bool;
}

pub fn has_accessibility_permission() -> bool {
    unsafe { AXIsProcessTrusted() }
}

pub struct MacOsTextInjector {
    enigo: Enigo,
}

impl MacOsTextInjector {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let enigo = Enigo::new(&Settings::default())?;
        Ok(Self { enigo })
    }
}

impl TextInjector for MacOsTextInjector {
    fn inject(&mut self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        if !has_accessibility_permission() {
            return Err("Accessibility permission is missing. Enable it in System Settings.".into());
        }
        // We use enigo to type the string sequence
        self.enigo.text(text)?;
        Ok(())
    }
}

