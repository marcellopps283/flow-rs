use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, hotkey::{HotKey, Code}};
use enigo::{Enigo, Keyboard, Settings};

pub enum AppEvent {
    ToggleListening,
    #[allow(dead_code)]
    TranscriptionDone(String),
}

pub fn init_automation(tx: tokio::sync::mpsc::Sender<AppEvent>) -> GlobalHotKeyManager {
    let manager = GlobalHotKeyManager::new().expect("Failed to initialize GlobalHotKeyManager");
    let hotkey = HotKey::new(None, Code::F8);
    
    manager.register(hotkey).expect("Failed to register F8 hotkey");
    
    let receiver = GlobalHotKeyEvent::receiver().clone();
    
    std::thread::spawn(move || {
        loop {
            if let Ok(event) = receiver.recv() {
                if event.id == hotkey.id() {
                    let _ = tx.blocking_send(AppEvent::ToggleListening);
                }
            }
        }
    });

    manager
}

pub fn type_text(text: &str) {
    let mut enigo = Enigo::new(&Settings::default()).expect("Failed to initialize Enigo");
    let _ = enigo.text(text);
}
