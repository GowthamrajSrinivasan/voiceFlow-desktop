use tauri::{Emitter, Manager};
use std::time::Duration;
use std::thread;
use std::sync::{Arc, Mutex};


use voiceflow_shared::config::settings::AppSettings;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;

use voiceflow_core::{VoiceFlow, RuntimeProfile, VoiceFlowEvent};
use voiceflow_desktop_hotkeys::{VoiceFlowHotKeyManager, GlobalHotKeyEvent, HotKeyState};
use voiceflow_desktop_text_injection::get_injector;

use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_notification::NotificationExt;

struct AppStatus {
    pub is_paused: Mutex<bool>,
}

#[tauri::command]
fn stop_listening(stop_tx: tauri::State<std::sync::mpsc::Sender<()>>) {
    let _ = stop_tx.send(());
}

#[tauri::command]
fn get_settings() -> AppSettings {
    AppSettings::load()
}

#[tauri::command]
fn save_settings(settings: AppSettings, app_handle: tauri::AppHandle) -> Result<(), String> {
    // 1. Save to disk
    settings.save().map_err(|e| e.to_string())?;

    // 2. Update Hotkey
    let hotkey_manager = app_handle.state::<Arc<Mutex<VoiceFlowHotKeyManager>>>();
    if let Err(e) = hotkey_manager.lock().unwrap().update_main_hotkey(&settings.hotkey) {
        eprintln!("Failed to update hotkey dynamically: {}", e);
    }

    // 3. Update Autostart
    let autostart_manager = app_handle.autolaunch();
    if settings.launch_at_login {
        let _ = autostart_manager.enable();
    } else {
        let _ = autostart_manager.disable();
    }

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--silently"])))
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![stop_listening, get_settings, save_settings])
        .setup(|app| {
            let app_handle = app.handle().clone();
            let window = app.get_webview_window("main").expect("no main window");
            
            let settings_window = app.get_webview_window("settings").expect("no settings window");
            let settings_window_clone = settings_window.clone();
            settings_window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = settings_window_clone.hide();
                }
            });

            app.manage(AppStatus {
                is_paused: Mutex::new(false),
            });

            // Tray menu setup
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let settings_i = MenuItem::with_id(app, "settings", "Settings...", true, None::<&str>)?;
            let pause_i = MenuItem::with_id(app, "pause_toggle", "Pause Dictation", true, None::<&str>)?;
            let about_i = MenuItem::with_id(app, "about", "About VoiceFlow", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&about_i, &pause_i, &settings_i, &quit_i])?;

            let pause_i_clone = pause_i.clone();
            let tray_icon = tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png"))?;
            
            let _tray = TrayIconBuilder::new()
                .icon(tray_icon)
                .menu(&menu)
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "quit" => app.exit(0),
                    "settings" => {
                        if let Some(settings_window) = app.get_webview_window("settings") {
                            let _ = settings_window.show();
                            let _ = settings_window.set_focus();
                        }
                    }
                    "pause_toggle" => {
                        let state = app.state::<AppStatus>();
                        let mut is_paused = state.is_paused.lock().unwrap();
                        *is_paused = !*is_paused;
                        let text = if *is_paused { "Resume Dictation" } else { "Pause Dictation" };
                        let _ = pause_i_clone.set_text(text);
                    }
                    _ => {}
                })
                .build(app)?;

            // Hotkey setup
            let initial_settings = AppSettings::load();
            let hotkey_manager = VoiceFlowHotKeyManager::new(&initial_settings.hotkey).unwrap_or_else(|_| {
                eprintln!("Failed to init hotkey manager with custom hotkey, falling back to Alt+Space");
                VoiceFlowHotKeyManager::new("Alt+Space").unwrap()
            });
            let hotkey_manager_arc = Arc::new(Mutex::new(hotkey_manager));
            app.manage(hotkey_manager_arc.clone());
            
            let (stop_tx, _stop_rx) = std::sync::mpsc::channel::<()>();
            app.manage(stop_tx);

            // Core Engine setup
            #[cfg(target_os = "macos")]
            let profile = RuntimeProfile::DesktopMac;
            #[cfg(target_os = "windows")]
            let profile = RuntimeProfile::DesktopWindows;
            #[cfg(not(any(target_os = "macos", target_os = "windows")))]
            let profile = RuntimeProfile::DesktopMac; // fallback

            let mut engine = VoiceFlow::new(profile);
            let event_receiver = engine.subscribe();

            // Store engine behind arc mutex if we need to call it from other threads
            let engine = Arc::new(Mutex::new(engine));
            let engine_clone = Arc::clone(&engine);

            // Trigger the background model prefetch 5s after startup
            engine.lock().unwrap().prefetch_model();

            // Hotkey Receiver Thread
            let receiver = GlobalHotKeyEvent::receiver();
            let hotkey_manager_clone = Arc::clone(&hotkey_manager_arc);
            thread::spawn(move || {
                loop {
                    if let Ok(event) = receiver.try_recv() {
                        let is_paused = *app_handle.state::<AppStatus>().is_paused.lock().unwrap();
                        let main_id = hotkey_manager_clone.lock().unwrap().main_hotkey_id();
                        
                        if !is_paused && event.state == HotKeyState::Pressed {
                            if event.id == main_id {
                                let engine = engine_clone.lock().unwrap();
                                if engine.is_listening() {
                                    engine.stop_listening();
                                } else {
                                    engine.start_listening();
                                }
                            }
                        }
                    }
                    thread::sleep(Duration::from_millis(50));
                }
            });

            // Event Subscriber Thread
            let app_handle = app.handle().clone();
            thread::spawn(move || {
                let mut injector = get_injector().expect("Failed to initialize text injector");
                
                loop {
                    if let Ok(event) = event_receiver.recv() {
                        match event {
                            VoiceFlowEvent::ListeningStarted => {
                                let settings = AppSettings::load();
                                if settings.overlay_enabled {
                                    let _ = window.show();
                                }
                                let _ = app_handle.emit("ListeningStarted", ());
                            }
                            VoiceFlowEvent::ListeningStopped => {
                                let _ = app_handle.emit("ListeningStopped", ());
                                let _ = window.hide();
                            }
                            VoiceFlowEvent::PartialTranscript(text) => {
                                let _ = app_handle.emit("PartialTranscript", text);
                            }
                            VoiceFlowEvent::FinalTranscript(text) => {
                                let _ = app_handle.emit("FinalTranscript", text.clone());
                                
                                // Hide overlay to restore focus to underlying window
                                let _ = window.hide();
                                thread::sleep(Duration::from_millis(300));
                                
                                let settings = AppSettings::load();
                                if settings.auto_paste {
                                    if let Err(e) = injector.inject(&text) {
                                        eprintln!("Injection failed: {}", e);
                                        if settings.clipboard_fallback {
                                            let _ = voiceflow_desktop_text_injection::copy_to_clipboard(&text);
                                        }
                                    }
                                } else {
                                    if settings.clipboard_fallback {
                                        let _ = voiceflow_desktop_text_injection::copy_to_clipboard(&text);
                                    }
                                }
                            }
                            VoiceFlowEvent::Error(err) => {
                                let _ = app_handle.emit("ErrorOccurred", err.clone());
                                if AppSettings::load().show_notifications {
                                    let _ = app_handle.notification().builder().title("VoiceFlow Error").body(&err).show();
                                }
                            }
                            VoiceFlowEvent::EngineInitializing => {
                                println!("Initializing VoiceFlow Engine...");
                                let _ = app_handle.emit("EngineInitializing", ());
                            }
                            VoiceFlowEvent::ModelDownloadStarted => {
                                println!("Model download started");
                                let _ = app_handle.emit("ModelDownloadStarted", ());
                                if AppSettings::load().show_notifications {
                                    let _ = app_handle.notification().builder()
                                        .title("VoiceFlow")
                                        .body("Downloading the latest AI dictation model. We'll be ready in a few seconds...")
                                        .show();
                                }
                            }
                            VoiceFlowEvent::ModelDownloading(percent) => {
                                println!("Downloading model: {}%", percent);
                                let _ = app_handle.emit("ModelDownloading", percent);
                            }
                            VoiceFlowEvent::ModelDownloadComplete => {
                                println!("Model download complete!");
                                let _ = app_handle.emit("ModelDownloadComplete", ());
                            }
                            VoiceFlowEvent::ModelLoading => {
                                println!("Loading Whisper Model into memory...");
                                let _ = app_handle.emit("ModelLoading", ());
                            }
                            VoiceFlowEvent::EngineReady => {
                                println!("VoiceFlow Engine is fully READY!");
                                let _ = app_handle.emit("EngineReady", ());
                                if AppSettings::load().show_notifications {
                                    let _ = app_handle.notification().builder().title("VoiceFlow").body("Dictation engine is ready to use!").show();
                                }
                            }
                            VoiceFlowEvent::EngineNotReady => {
                                eprintln!("Dictation blocked: Engine is not ready yet!");
                                let _ = app_handle.emit("EngineNotReady", ());
                            }
                        }
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
