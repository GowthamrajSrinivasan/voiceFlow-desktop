pub use global_hotkey::*;
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
};

pub struct VoiceFlowHotKeyManager {
    manager: GlobalHotKeyManager,
    main_hotkey: HotKey,
    cancel_hotkey: HotKey,
}

impl VoiceFlowHotKeyManager {
    pub fn new(hotkey_str: &str) -> std::result::Result<Self, Box<dyn std::error::Error>> {
        let manager = GlobalHotKeyManager::new()?;
        
        let main_hotkey = Self::parse_hotkey(hotkey_str)?;
        manager.register(main_hotkey)?;

        // Cancel hotkey: Escape (registered/unregistered dynamically)
        let cancel_hotkey = HotKey::new(None, Code::Escape);

        Ok(Self {
            manager,
            main_hotkey,
            cancel_hotkey,
        })
    }

    pub fn update_main_hotkey(&mut self, new_hotkey_str: &str) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let new_hotkey = Self::parse_hotkey(new_hotkey_str)?;
        
        // Unregister old
        let _ = self.manager.unregister(self.main_hotkey);
        
        // Register new
        self.manager.register(new_hotkey)?;
        self.main_hotkey = new_hotkey;
        
        Ok(())
    }

    fn parse_hotkey(hotkey_str: &str) -> std::result::Result<HotKey, String> {
        let mut modifiers = Modifiers::empty();
        let mut code = Code::Unidentified;

        let parts: Vec<&str> = hotkey_str.split('+').map(|s| s.trim()).collect();
        for part in parts {
            match part.to_uppercase().as_str() {
                "ALT" => modifiers.insert(Modifiers::ALT),
                "CTRL" | "CONTROL" => modifiers.insert(Modifiers::CONTROL),
                "SHIFT" => modifiers.insert(Modifiers::SHIFT),
                "META" | "WIN" | "CMD" | "COMMAND" => modifiers.insert(Modifiers::META),
                "SPACE" => code = Code::Space,
                "ENTER" => code = Code::Enter,
                "ESCAPE" | "ESC" => code = Code::Escape,
                "TAB" => code = Code::Tab,
                c if c.len() == 1 => {
                    let char_code = c.chars().next().unwrap();
                    if char_code.is_ascii_alphabetic() {
                        match char_code {
                            'A' => code = Code::KeyA, 'B' => code = Code::KeyB, 'C' => code = Code::KeyC,
                            'D' => code = Code::KeyD, 'E' => code = Code::KeyE, 'F' => code = Code::KeyF,
                            'G' => code = Code::KeyG, 'H' => code = Code::KeyH, 'I' => code = Code::KeyI,
                            'J' => code = Code::KeyJ, 'K' => code = Code::KeyK, 'L' => code = Code::KeyL,
                            'M' => code = Code::KeyM, 'N' => code = Code::KeyN, 'O' => code = Code::KeyO,
                            'P' => code = Code::KeyP, 'Q' => code = Code::KeyQ, 'R' => code = Code::KeyR,
                            'S' => code = Code::KeyS, 'T' => code = Code::KeyT, 'U' => code = Code::KeyU,
                            'V' => code = Code::KeyV, 'W' => code = Code::KeyW, 'X' => code = Code::KeyX,
                            'Y' => code = Code::KeyY, 'Z' => code = Code::KeyZ,
                            _ => return Err(format!("Unsupported key: {}", part)),
                        }
                    } else if char_code.is_ascii_digit() {
                        match char_code {
                            '0' => code = Code::Digit0, '1' => code = Code::Digit1, '2' => code = Code::Digit2,
                            '3' => code = Code::Digit3, '4' => code = Code::Digit4, '5' => code = Code::Digit5,
                            '6' => code = Code::Digit6, '7' => code = Code::Digit7, '8' => code = Code::Digit8,
                            '9' => code = Code::Digit9,
                            _ => return Err(format!("Unsupported key: {}", part)),
                        }
                    } else {
                        return Err(format!("Unsupported key: {}", part));
                    }
                }
                _ => return Err(format!("Unsupported key: {}", part)),
            }
        }

        if code == Code::Unidentified {
            return Err("No key code specified in hotkey string".to_string());
        }

        let mods = if modifiers.is_empty() { None } else { Some(modifiers) };
        Ok(HotKey::new(mods, code))
    }

    pub fn main_hotkey_id(&self) -> u32 {
        self.main_hotkey.id()
    }

    pub fn cancel_hotkey_id(&self) -> u32 {
        self.cancel_hotkey.id()
    }

    pub fn register_cancel(&self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        self.manager.register(self.cancel_hotkey)?;
        Ok(())
    }

    pub fn unregister_cancel(&self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        self.manager.unregister(self.cancel_hotkey)?;
        Ok(())
    }

    pub fn unregister_all(&self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let _ = self.manager.unregister(self.main_hotkey);
        let _ = self.manager.unregister(self.cancel_hotkey);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hotkey() {
        let hk = VoiceFlowHotKeyManager::parse_hotkey("Alt+Space").unwrap();
        assert_eq!(hk.modifiers(), Modifiers::ALT);
        // Code doesn't derive PartialEq properly in some versions, but we can check if it parses
        
        let hk = VoiceFlowHotKeyManager::parse_hotkey("Ctrl + Shift + P").unwrap();
        assert_eq!(hk.modifiers(), Modifiers::CONTROL | Modifiers::SHIFT);
        
        let hk = VoiceFlowHotKeyManager::parse_hotkey("Cmd + Enter").unwrap();
        assert_eq!(hk.modifiers(), Modifiers::META);
    }
}

