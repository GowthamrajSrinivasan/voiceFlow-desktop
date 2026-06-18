#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "windows")]
pub mod windows;
pub mod permissions;

pub trait TextInjector {
    fn inject(&mut self, text: &str) -> Result<(), Box<dyn std::error::Error>>;
}

#[cfg(target_os = "macos")]
pub fn get_injector() -> Result<Box<dyn TextInjector>, Box<dyn std::error::Error>> {
    Ok(Box::new(macos::MacOsTextInjector::new()?))
}

#[cfg(target_os = "windows")]
pub fn get_injector() -> Result<Box<dyn TextInjector>, Box<dyn std::error::Error>> {
    Ok(Box::new(windows::WindowsTextInjector::new()?))
}

pub fn copy_to_clipboard(text: &str) -> Result<(), Box<dyn std::error::Error>> {
    use std::process::{Command, Stdio};
    use std::io::Write;

    #[cfg(target_os = "macos")]
    let cmd = "pbcopy";
    #[cfg(target_os = "windows")]
    let cmd = "clip";
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let cmd = "xclip";

    let mut child = Command::new(cmd)
        .stdin(Stdio::piped())
        .spawn()?;
    
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes())?;
    }
    
    child.wait()?;
    Ok(())
}

