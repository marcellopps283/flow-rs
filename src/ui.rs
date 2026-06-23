use eframe::egui;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::runtime::Runtime;
use tray_icon::{TrayIcon, TrayIconEvent};

pub struct FlowApp {
    #[allow(dead_code)]
    rt: Arc<Runtime>,
    #[allow(dead_code)]
    is_listening_state: Arc<AtomicBool>,
    current_amplitude: Arc<std::sync::atomic::AtomicU32>,
    _tray_icon: Option<TrayIcon>,
    menu_open: bool,
    menu_pos: egui::Pos2,
    stems: Vec<f32>,
    target_stems: Vec<f32>,
    time: f64,
    has_positioned: bool,
}

impl FlowApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, rt: Arc<Runtime>, is_listening_state: Arc<AtomicBool>, current_amplitude: Arc<std::sync::atomic::AtomicU32>, tray_icon: Option<TrayIcon>) -> Self {
        Self {
            rt,
            is_listening_state,
            current_amplitude,
            _tray_icon: tray_icon,
            menu_open: false,
            menu_pos: egui::pos2(0.0, 0.0),
            stems: vec![0.0; 15],
            target_stems: vec![0.0; 15],
            time: 0.0,
            has_positioned: false,
        }
    }
}

impl eframe::App for FlowApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Color32::TRANSPARENT.to_normalized_gamma_f32()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Tray icon logic
        if self._tray_icon.is_some() {
            if let Ok(event) = TrayIconEvent::receiver().try_recv() {
                if let tray_icon::TrayIconEvent::Click { button, button_state, position, .. } = event {
                    if button == tray_icon::MouseButton::Right && button_state == tray_icon::MouseButtonState::Up {
                        self.menu_open = !self.menu_open;
                        self.menu_pos = egui::pos2(position.x as f32, position.y as f32 - 130.0);
                    }
                }
            }
        }

        if self._tray_icon.is_some() && self.menu_open {
            let viewport_id = egui::ViewportId::from_hash_of("shadcn_menu");
            
            let viewport_builder = egui::ViewportBuilder::default()
                .with_title("Menu")
                .with_inner_size([160.0, 110.0])
                .with_position(self.menu_pos)
                .with_decorations(false)
                .with_transparent(true)
                .with_always_on_top();

            let mut should_close = false;

            ctx.show_viewport_immediate(viewport_id, viewport_builder, |ctx, _class| {
                let frame = egui::Frame::none()
                    .fill(egui::Color32::from_rgba_premultiplied(15, 15, 15, 240))
                    .rounding(8.0)
                    .inner_margin(8.0)
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(40)));
                
                egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
                    ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                    ui.style_mut().visuals.widgets.hovered.bg_fill = egui::Color32::from_gray(40);
                    ui.style_mut().visuals.widgets.hovered.rounding = egui::Rounding::same(6.0);
                    ui.style_mut().visuals.widgets.active.bg_fill = egui::Color32::from_gray(60);
                    ui.style_mut().visuals.widgets.active.rounding = egui::Rounding::same(6.0);
                    
                    ui.label(egui::RichText::new("Flow AI").strong().color(egui::Color32::WHITE).size(14.0));
                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);
                    
                    let btn_size = egui::vec2(ui.available_width(), 26.0);
                    
                    if ui.add_sized(btn_size, egui::Button::new(egui::RichText::new("Settings").color(egui::Color32::from_gray(200)))).clicked() {
                        should_close = true;
                    }
                    ui.add_space(2.0);
                    if ui.add_sized(btn_size, egui::Button::new(egui::RichText::new("Quit").color(egui::Color32::from_rgb(255, 100, 100)))).clicked() {
                        std::process::exit(0);
                    }
                });

                if ctx.input(|i| i.pointer.any_click() && !ctx.is_pointer_over_area()) {
                    should_close = true;
                }
            });

            if should_close {
                self.menu_open = false;
            }
        }

        // Pill logic
        if !self.has_positioned {
            if let Some(monitor_size) = ctx.input(|i| i.viewport().monitor_size) {
                let window_width = 180.0;
                let pos_x = (monitor_size.x - window_width) / 2.0;
                let pos_y = 10.0;
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(pos_x, pos_y)));
                self.has_positioned = true;
            }
        }

        let dt = ctx.input(|i| i.stable_dt) as f32;
        self.time += dt as f64;

        let mut mic_amp = f32::from_bits(self.current_amplitude.load(Ordering::Relaxed)) * 2.5; // Boost sensitivity
        // Add a bit of noise/movement even if completely quiet so it feels alive
        let base_noise = (self.time as f32 * 5.0).sin().abs() * 0.15;
        mic_amp = mic_amp.max(base_noise);

        for i in 0..15 {
            if self.is_listening_state.load(Ordering::Relaxed) {
                // Vary each stem slightly using the mic amplitude and some phase
                let phase = (self.time as f32 * 3.0 + i as f32 * 0.4).sin().abs() * 0.4 + 0.6;
                self.target_stems[i] = (mic_amp * phase) + 0.1;
            } else {
                // Idle animation
                let idle_sine = (self.time as f32 * 1.5 + i as f32 * 0.3).sin().abs() * 0.2;
                self.target_stems[i] = idle_sine + 0.15;
            }
            self.stems[i] += (self.target_stems[i] - self.stems[i]) * dt * 15.0;
        }

        let width = if self.is_listening_state.load(Ordering::Relaxed) { 180.0 } else { 80.0 };
        let height = if self.is_listening_state.load(Ordering::Relaxed) { 40.0 } else { 28.0 };

        let frame = egui::Frame::none();

        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            let center = ui.max_rect().center();
            let rect = egui::Rect::from_center_size(center, egui::vec2(width, height));

            ui.painter().rect_filled(
                rect,
                rect.height() / 2.0,
                egui::Color32::from_black_alpha(200),
            );

            if self.is_listening_state.load(Ordering::Relaxed) {
                let num_stems = 15;
                let spacing = 3.0;
                let stem_width = (rect.width() - (spacing * (num_stems as f32 + 1.0)) - 20.0) / num_stems as f32;
                let mut x = rect.left() + 10.0 + spacing;

                for i in 0..num_stems {
                    let amplitude = self.stems[i];
                    let stem_height = (amplitude * rect.height() * 0.8).clamp(4.0, rect.height() - 8.0);
                    
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
                        stem_width / 2.0,
                        color,
                    );

                    x += stem_width + spacing;
                }
            } else {
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
        
        ctx.request_repaint();
    }
}

