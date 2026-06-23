use eframe::egui;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::runtime::Runtime;

pub struct FlowApp {
    #[allow(dead_code)]
    rt: Arc<Runtime>,
    is_listening_state: Arc<AtomicBool>,
    stems: Vec<f32>,
    target_stems: Vec<f32>,
    time: f64,
}

impl FlowApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, rt: Arc<Runtime>, is_listening_state: Arc<AtomicBool>) -> Self {
        Self {
            rt,
            is_listening_state,
            stems: vec![0.0; 15],
            target_stems: vec![0.0; 15],
            time: 0.0,
        }
    }
}

impl eframe::App for FlowApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Color32::TRANSPARENT.to_normalized_gamma_f32()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let dt = ctx.input(|i| i.stable_dt) as f32;
        self.time += dt as f64;

        // Animate stems
        for i in 0..15 {
            if self.is_listening_state.load(Ordering::Relaxed) {
                // Simulate an idle sine-wave motion for the stems
                let base_sine = (self.time as f32 * 2.0 + i as f32 * 0.5).sin().abs() * 0.3;
                self.target_stems[i] = base_sine + 0.1;
            } else {
                self.target_stems[i] = 0.05;
            }
            // Smooth interpolation towards the target
            self.stems[i] += (self.target_stems[i] - self.stems[i]) * dt * 10.0;
        }

        let width = if self.is_listening_state.load(Ordering::Relaxed) { 180.0 } else { 80.0 };
        let height = if self.is_listening_state.load(Ordering::Relaxed) { 40.0 } else { 28.0 };

        let frame = egui::Frame::none();

        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            // Allocate space for the Dynamic Island
            let center = ui.max_rect().center();
            let rect = egui::Rect::from_center_size(center, egui::vec2(width, height));

            // Draw the black pill background
            ui.painter().rect_filled(
                rect,
                rect.height() / 2.0, // Perfect pill rounding
                egui::Color32::from_black_alpha(200),
            );

            if self.is_listening_state.load(Ordering::Relaxed) {
                // Draw neon stems inside the pill
                let num_stems = 15;
                let spacing = 3.0;
                let stem_width = (rect.width() - (spacing * (num_stems as f32 + 1.0)) - 20.0) / num_stems as f32;
                let mut x = rect.left() + 10.0 + spacing;

                for i in 0..num_stems {
                    let amplitude = self.stems[i];
                    let stem_height = (amplitude * rect.height() * 0.8).clamp(4.0, rect.height() - 8.0);
                    
                    // Vibrant gradient based on horizontal position (Cyan -> Magenta -> Purple)
                    let t = i as f32 / num_stems as f32;
                    let color = if t < 0.33 {
                        egui::Color32::from_rgb(0, 255, 255)
                    } else if t < 0.66 {
                        egui::Color32::from_rgb(255, 0, 255)
                    } else {
                        egui::Color32::from_rgb(128, 0, 255)
                    };

                    let stem_rect = egui::Rect::from_center_size(
                        egui::pos2(x + stem_width / 2.0, rect.center().y),
                        egui::vec2(stem_width, stem_height),
                    );

                    ui.painter().rect_filled(
                        stem_rect,
                        stem_width / 2.0, // Round the stems
                        color,
                    );

                    x += stem_width + spacing;
                }
            } else {
                // Just draw a small label when idle
                let label_pos = egui::pos2(rect.center().x - 12.0, rect.center().y - 6.0);
                ui.painter().text(
                    label_pos,
                    egui::Align2::LEFT_TOP,
                    "Flow",
                    egui::FontId::proportional(12.0),
                    egui::Color32::WHITE,
                );
            }
        });
        
        ctx.request_repaint(); // 60 FPS animation
    }
}
