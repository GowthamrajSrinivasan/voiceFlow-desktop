use crate::TextInjector;
use enigo::{Enigo, Keyboard, Settings};

pub struct WindowsTextInjector {
    enigo: Enigo,
}

impl WindowsTextInjector {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let enigo = Enigo::new(&Settings::default())?;
        Ok(Self { enigo })
    }
}

impl TextInjector for WindowsTextInjector {
    fn inject(&mut self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.enigo.text(text)?;
        Ok(())
    }
}
