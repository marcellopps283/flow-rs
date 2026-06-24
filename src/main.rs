// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

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
use tray_icon::TrayIconBuilder;
use tray_icon::menu::{Menu, MenuItem};

fn create_tray_icon_rgba() -> (Vec<u8>, u32, u32) {
    let size: u32 = 32;
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    let cx = size as f32 / 2.0;
    let cy = size as f32 / 2.0;
    let bg_radius = cx - 1.0;
    let pi = std::f32::consts::PI;

    for y in 0..size {
        for x in 0..size {
            let idx = ((y * size + x) * 4) as usize;
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();

            // Rounded black background
            if dist <= bg_radius {
                rgba[idx] = 15;
                rgba[idx + 1] = 15;
                rgba[idx + 2] = 15;
                rgba[idx + 3] = 255;

                // Waveform: draw a sine wave across the icon horizontally
                // Normalize x to 0..1 range within the circle
                let nx = (x as f32 - 4.0) / (size as f32 - 8.0); // padding of 4px
                if nx >= 0.0 && nx <= 1.0 {
                    // Sine wave with varying amplitude (louder in the middle)
                    let amplitude_envelope = (nx * pi).sin(); // envelope: peak at center
                    let wave_y = cy + (nx * pi * 3.0).sin() * amplitude_envelope * 8.0;

                    // Draw with thickness of ~2.5px
                    let dist_to_wave = (y as f32 - wave_y).abs();
                    if dist_to_wave < 1.8 {
                        rgba[idx] = 255;
                        rgba[idx + 1] = 255;
                        rgba[idx + 2] = 255;
                        rgba[idx + 3] = 255;
                    }
                }
            }
        }
    }
    (rgba, size, size)
}

fn main() -> Result<(), eframe::Error> {
    dotenvy::dotenv().ok();

    let rt = Arc::new(Runtime::new().expect("Failed to create Tokio runtime"));

    let (tx_app_event, mut rx_app_event) = tokio_mpsc::channel(100);
    // MUST keep _hotkey_manager alive on the main thread, or the hotkey gets unregistered!
    let _hotkey_manager = automation::init_automation(tx_app_event);

    let is_listening_state = Arc::new(AtomicBool::new(false));
    let is_listening_clone = is_listening_state.clone();

    let current_amplitude = Arc::new(std::sync::atomic::AtomicU32::new(0.0f32.to_bits()));
    let current_amplitude_ui = current_amplitude.clone();

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
                            println!("Processing audio... samples: {}, rate: {}, channels: {}", audio_buffer.len(), sample_rate, channels);
                            
                            // Resample to 16kHz mono for Nemotron
                            let mono_samples: Vec<f32> = if channels > 1 {
                                audio_buffer.chunks(channels as usize)
                                    .map(|chunk| chunk.iter().sum::<f32>() / channels as f32)
                                    .collect()
                            } else {
                                audio_buffer.clone()
                            };

                            let target_rate: u32 = 16000;
                            let resampled = if sample_rate != target_rate {
                                let ratio = sample_rate as f64 / target_rate as f64;
                               let new_len = (mono_samples.len() as f64 / ratio) as usize;
                                let mut out = Vec::with_capacity(new_len);
                                for i in 0..new_len {
                                    let src_idx = i as f64 * ratio;
                                    let idx0 = src_idx.floor() as usize;
                                    let idx1 = (idx0 + 1).min(mono_samples.len() - 1);
                                    let frac = (src_idx - idx0 as f64) as f32;
                                    out.push(mono_samples[idx0] * (1.0 - frac) + mono_samples[idx1] * frac);
                                }
                                out
                            } else {
                                mono_samples
                            };

                            println!("Resampled to {} samples at 16kHz mono", resampled.len());

                            match ai_pipeline.transcribe_audio(&resampled, target_rate, 1) {
                                Ok(raw_text) => {
                                    println!("Raw Transcribed: {}", raw_text);
                                    if !raw_text.trim().is_empty() {
                                        match rt_clone.block_on(ai_pipeline.polish_text(&raw_text)) {
                                            Ok(polished_text) => {
                                                println!("Polished: {}", polished_text);
                                                automation::type_text(&polished_text);
                                            }
                                            Err(e) => println!("Groq API error: {}", e),
                                        }
                                    } else {
                                        println!("Audio was empty or silent.");
                                    }
                                }
                                Err(e) => println!("Transcription error: {}", e),
                            }
                        }
                    }
                    _ => {}
                }
            }

            if is_listening {
                if let Some(rx) = &audio_rx {
                    let mut max_abs: f32 = 0.0;
                    let mut count = 0;
                    while let Ok(sample) = rx.try_recv() {
                        audio_buffer.push(sample);
                        let abs = sample.abs();
                        if abs > max_abs { max_abs = abs; }
                        count += 1;
                    }
                    if count > 0 {
                        current_amplitude.store((max_abs * 2.0).clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
                    } else {
                        let prev = f32::from_bits(current_amplitude.load(Ordering::Relaxed));
                        current_amplitude.store((prev * 0.8).to_bits(), Ordering::Relaxed);
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
            .with_inner_size([300.0, 70.0])
            .with_taskbar(false),
        ..Default::default()
    };

    eframe::run_native(
        "Flow 2.0",
        options,
        Box::new(|cc| {
            // Build tray icon inside the event loop closure where event loop is active
            let (icon_rgba, icon_w, icon_h) = create_tray_icon_rgba();
            
            let tray_menu = Menu::new();
            let quit_i = MenuItem::new("Quit", true, None);
            let _ = tray_menu.append(&quit_i);
            let quit_id = quit_i.id().clone();

            let tray_icon = match TrayIconBuilder::new()
                .with_tooltip("Flow AI")
                .with_menu(Box::new(tray_menu.clone()))
                .with_icon(tray_icon::Icon::from_rgba(icon_rgba, icon_w, icon_h).expect("Failed to create icon"))
                .build()
            {
                Ok(icon) => {
                    println!("Tray icon created successfully inside run_native!");
                    Some(icon)
                }
                Err(e) => {
                    println!("WARNING: Tray icon failed inside run_native ({}), continuing without it.", e);
                    None
                }
            };
            Box::new(ui::FlowApp::new(cc, rt, is_listening_state, current_amplitude_ui, tray_icon, quit_id, tray_menu, quit_i))
        }),
    )
}

