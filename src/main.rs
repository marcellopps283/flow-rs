#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ui;
mod audio;
mod automation;
mod ai;

use eframe::egui;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::runtime::Runtime;
use tokio::sync::mpsc as tokio_mpsc;
use std::sync::mpsc as std_mpsc;

fn main() -> Result<(), eframe::Error> {
    dotenvy::dotenv().ok();

    let rt = Arc::new(Runtime::new().expect("Failed to create Tokio runtime"));

    let (tx_app_event, mut rx_app_event) = tokio_mpsc::channel(100);
    // MUST keep _hotkey_manager alive on the main thread, or the hotkey gets unregistered!
    let _hotkey_manager = automation::init_automation(tx_app_event);

    let is_listening_state = Arc::new(AtomicBool::new(false));
    let is_listening_clone = is_listening_state.clone();

    let rt_clone = rt.clone();
    std::thread::spawn(move || {
        let mut ai_pipeline = ai::AiPipeline::new().expect("Failed to init AI");
        let mut audio_recorder = audio::AudioRecorder::new();
        
        let mut is_listening = false;
        let mut audio_rx: Option<std_mpsc::Receiver<f32>> = None;
        let mut audio_buffer = Vec::new();

        loop {
            if let Ok(event) = rx_app_event.try_recv() {
                match event {
                    automation::AppEvent::ToggleListening => {
                        is_listening = !is_listening;
                        is_listening_clone.store(is_listening, Ordering::Relaxed);
                        
                        if is_listening {
                            let (audio_tx, rx) = std_mpsc::channel();
                            audio_rx = Some(rx);
                            audio_buffer.clear();
                            let _ = audio_recorder.start_recording(audio_tx);
                        } else {
                            audio_recorder.stop_recording();
                            audio_rx = None;
                            
                            // Transcription and Polishing Pipeline
                            let sample_rate = audio_recorder.sample_rate;
                            let channels = audio_recorder.channels;
                            if let Ok(raw_text) = ai_pipeline.transcribe_audio(&audio_buffer, sample_rate, channels) {
                                if let Ok(polished_text) = rt_clone.block_on(ai_pipeline.polish_text(&raw_text)) {
                                    automation::type_text(&polished_text);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }

            if is_listening {
                if let Some(rx) = &audio_rx {
                    while let Ok(sample) = rx.try_recv() {
                        audio_buffer.push(sample);
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    });

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_inner_size([180.0, 40.0])
            .with_taskbar(false),
        ..Default::default()
    };

    eframe::run_native(
        "Flow 2.0",
        options,
        Box::new(|cc| Box::new(ui::FlowApp::new(cc, rt, is_listening_state))),
    )
}
