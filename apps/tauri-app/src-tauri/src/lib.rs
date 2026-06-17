use tauri::Emitter;
use std::time::Duration;
use std::thread;
use std::sync::{Arc, Mutex};
use global_hotkey::GlobalHotKeyEvent;
use voiceflow_core::hotkey::VoiceFlowHotKeyManager;
use voiceflow_core::audio_capture::AudioCapture;
use voiceflow_core::stt::{SpeechRecognizer, WhisperCppRecognizer};
use voiceflow_core::pipeline::vocabulary::{VocabularyEngine, VocabularyItem};
use voiceflow_core::pipeline::formatting::FormattingEngine;
use voiceflow_core::injection::get_injector;
use tauri::Manager;
use voiceflow_shared::config::settings::AppSettings;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;

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
fn save_settings(settings: AppSettings) -> Result<(), String> {
    settings.save().map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![stop_listening, get_settings, save_settings])
        .setup(|app| {
            let app_handle = app.handle().clone();
            let window = app.get_webview_window("main").expect("no main window");
            
            // Intercept settings window close to hide it (keep running in background)
            let settings_window = app.get_webview_window("settings").expect("no settings window");
            let settings_window_clone = settings_window.clone();
            settings_window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = settings_window_clone.hide();
                }
            });

            // Initialize AppStatus state (to track pause/resume dictation)
            app.manage(AppStatus {
                is_paused: Mutex::new(false),
            });

            // Setup Tray Menu
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
                    "quit" => {
                        app.exit(0);
                    }
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
                    "about" => {
                        let app_version = env!("CARGO_PKG_VERSION");
                        let model_path = "/Users/gowthamrajsrinivasan/Documents/Projects/Flow/models/ggml-small.en.bin";
                        let msg = format!("VoiceFlow v{}\nModel: {}\nRAM: ~500MB (Whisper small.en)", app_version, model_path);
                        let cmd = format!("display dialog \"{}\" with title \"About VoiceFlow\" buttons {{\"OK\"}} default button \"OK\" with icon note", msg);
                        let _ = std::process::Command::new("osascript")
                            .arg("-e")
                            .arg(&cmd)
                            .spawn();
                    }
                    _ => {}
                })
                .build(app)?;

            // Initialize Hotkey Manager
            let hotkey_manager = VoiceFlowHotKeyManager::new().expect("Failed to init hotkey manager");
            app.manage(hotkey_manager);
            
            // UI Stop Channel
            let (stop_tx, stop_rx) = std::sync::mpsc::channel::<()>();
            app.manage(stop_tx);

            // Initialize Engines
            let model_path = "/Users/gowthamrajsrinivasan/Documents/Projects/Flow/models/ggml-small.en.bin";
            let whisper_recognizer = Arc::new(Mutex::new(
                WhisperCppRecognizer::new(model_path).expect("Failed to load Whisper model")
            ));

            let receiver = GlobalHotKeyEvent::receiver();

            thread::spawn(move || {
                let vocab_items = vec![
                    VocabularyItem {
                        canonical: "Requill".to_string(),
                        aliases: vec!["Requel".to_string(), "Requil".to_string(), "Re Quill".to_string()],
                    }
                ];
                let vocab_engine = VocabularyEngine::new(vocab_items);
                let format_engine = FormattingEngine::new();
                let mut injector = get_injector().expect("Failed to initialize text injector");

                let hotkey_manager = app_handle.state::<VoiceFlowHotKeyManager>();
                let main_id = hotkey_manager.main_hotkey_id();
                let cancel_id = hotkey_manager.cancel_hotkey_id();

                let mut is_recording = false;
                let mut audio_capture: Option<AudioCapture> = None;
                let mut partial_display_text = String::new();
                let mut last_partial_time = std::time::Instant::now();

                loop {
                    let mut should_toggle = false;
                    let mut should_cancel = false;

                    if let Ok(event) = receiver.try_recv() {
                        let is_paused = *app_handle.state::<AppStatus>().is_paused.lock().unwrap();
                        if !is_paused && event.state == global_hotkey::HotKeyState::Pressed {
                            if event.id == main_id {
                                should_toggle = true;
                            } else if event.id == cancel_id {
                                should_cancel = true;
                            }
                        }
                    }

                    if let Ok(_) = stop_rx.try_recv() {
                        if is_recording {
                            should_toggle = true;
                        }
                    }

                    if should_cancel && is_recording {
                        is_recording = false;
                        let _ = hotkey_manager.unregister_cancel();
                        let _ = app_handle.emit("ListeningStopped", ());
                        
                        // Discard recorded audio
                        let _ = audio_capture.take();
                        
                        // Clear/reset recognizer stream
                        if let Ok(mut recognizer) = whisper_recognizer.lock() {
                            recognizer.start_stream();
                        }
                        
                        // Hide window
                        let _ = window.hide();
                    } else if should_toggle {
                        is_recording = !is_recording;

                        if is_recording {
                            // Show overlay BEFORE emitting, but don't steal focus (only if overlay is enabled)
                            let settings = AppSettings::load();
                            if settings.overlay_enabled {
                                let _ = window.show();
                            }
                            
                            let _ = app_handle.emit("ListeningStarted", ());
                            partial_display_text.clear();
                            last_partial_time = std::time::Instant::now();
                            let mut recognizer = whisper_recognizer.lock().unwrap();
                            recognizer.start_stream();
                            
                            // Dynamically register Escape as cancel hotkey during recording
                            let _ = hotkey_manager.register_cancel();
                            
                            match AudioCapture::new() {
                                Ok(capture) => {
                                    audio_capture = Some(capture);
                                }
                                Err(e) => {
                                    let _ = app_handle.emit("ErrorOccurred", format!("Mic error: {}", e));
                                    let _ = hotkey_manager.unregister_cancel();
                                    is_recording = false;
                                }
                            }
                        } else {
                            let _ = app_handle.emit("ListeningStopped", ());
                            let _ = hotkey_manager.unregister_cancel();
                            
                            if let Some(mut capture) = audio_capture.take() {
                                let audio_data = capture.read_audio();
                                
                                let mut recognizer = whisper_recognizer.lock().unwrap();
                                recognizer.process_audio(&audio_data);
                                let mut text = recognizer.final_result();
                                eprintln!("[DEBUG] raw whisper output: {:?}", text);
                                
                                if !text.is_empty() {
                                    text = vocab_engine.apply(&text);
                                    text = format_engine.apply(&text);
                                    eprintln!("[DEBUG] formatted text: {:?}", text);
                                    
                                    let _ = app_handle.emit("FinalTranscript", text.clone());
                                    
                                    let _ = app_handle.emit("InjectionStarted", ());
                                    
                                    // Small delay so the UI can hide first, restoring focus
                                    thread::sleep(Duration::from_millis(300));
                                    
                                    let settings = AppSettings::load();
                                    match injector.inject(&text) {
                                        Ok(_) => {
                                            let _ = app_handle.emit("InjectionCompleted", ());
                                        }
                                        Err(e) => {
                                            eprintln!("[DEBUG] injection failed: {:?}", e);
                                            if settings.clipboard_fallback {
                                                match voiceflow_core::injection::copy_to_clipboard(&text) {
                                                    Ok(_) => {
                                                        let _ = app_handle.emit(
                                                            "ErrorOccurred",
                                                            "Copied to clipboard (No focus / Perm missing)".to_string()
                                                        );
                                                    }
                                                    Err(copy_err) => {
                                                        let _ = app_handle.emit(
                                                            "ErrorOccurred",
                                                            format!("Injection failed. Clipboard error: {}", copy_err)
                                                        );
                                                    }
                                                }
                                            } else {
                                                let _ = app_handle.emit("ErrorOccurred", format!("Injection failed: {}", e));
                                            }
                                        }
                                    }
                                } else {
                                    let _ = app_handle.emit("ErrorOccurred", "No speech detected");
                                }
                                
                                // Hide overlay after injection
                                thread::sleep(Duration::from_millis(1500));
                                let _ = window.hide();
                            }
                        }
                    }

                    if is_recording {
                        if let Some(capture) = audio_capture.as_mut() {
                            let new_audio = capture.read_audio();
                            if !new_audio.is_empty() {
                                let mut recognizer = whisper_recognizer.lock().unwrap();
                                recognizer.process_audio(&new_audio);
                            }
                        }

                        // 3 seconds gives Whisper enough context to produce real words
                        if last_partial_time.elapsed() >= Duration::from_millis(3000) {
                            last_partial_time = std::time::Instant::now();
                            let mut recognizer = whisper_recognizer.lock().unwrap();
                            if let Some(chunk_text) = recognizer.partial_result() {
                                if !chunk_text.is_empty() {
                                    if !partial_display_text.is_empty() {
                                        partial_display_text.push(' ');
                                    }
                                    partial_display_text.push_str(&chunk_text);
                                    let _ = app_handle.emit("PartialTranscript", partial_display_text.clone());
                                }
                            }
                        }
                    }

                    thread::sleep(Duration::from_millis(20));
                }
            });
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
