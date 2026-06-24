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
    frames_since_menu_open: usize,
    stems: Vec<f32>,
    target_stems: Vec<f32>,
    time: f64,
    has_positioned: bool,
    ui_width: f32,
    ui_height: f32,
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
            frames_since_menu_open: 0,
            stems: vec![0.0; 15],
            target_stems: vec![0.0; 15],
            time: 0.0,
            has_positioned: false,
            ui_width: 100.0,
            ui_height: 8.0,
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
            while let Ok(event) = TrayIconEvent::receiver().try_recv() {
                if let tray_icon::TrayIconEvent::Click { button, button_state, position, .. } = event {
                    if (button == tray_icon::MouseButton::Right || button == tray_icon::MouseButton::Left) 
                       && button_state == tray_icon::MouseButtonState::Down {
                        self.menu_open = !self.menu_open;
                        self.menu_pos = egui::pos2(position.x as f32, position.y as f32 - 130.0);
                        self.frames_since_menu_open = 0;
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

                if self.frames_since_menu_open > 5 {
                    if ctx.input(|i| i.pointer.any_pressed() && !ctx.is_pointer_over_area()) {
                        should_close = true;
                    }
                }
                self.frames_since_menu_open += 1;
            });

            if should_close {
                self.menu_open = false;
            }
        }

        // Pill logic
        if !self.has_positioned {
            if let Some(monitor_size) = ctx.input(|i| i.viewport().monitor_size) {
                let window_width = 300.0;
                let pos_x = (monitor_size.x - window_width) / 2.0;
                let pos_y = 1.0; // Glue to the very top edge, offset by 1px to prevent Windows snapping issues
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

        let target_width = if self.is_listening_state.load(Ordering::Relaxed) { 280.0 } else { 180.0 };
        let target_height = if self.is_listening_state.load(Ordering::Relaxed) { 60.0 } else { 12.0 };
        
        self.ui_width += (target_width - self.ui_width) * (dt * 15.0).min(1.0);
        self.ui_height += (target_height - self.ui_height) * (dt * 15.0).min(1.0);

        let frame = egui::Frame::none();

        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            let max_rect = ui.max_rect();
            let rect_width = self.ui_width;
            let rect_height = self.ui_height;
            // Align center horizontally, glue to top vertically
            let center_x = max_rect.center().x;
            let rect = egui::Rect::from_min_size(
                egui::pos2(center_x - rect_width / 2.0, 0.0),
                egui::vec2(rect_width, rect_height)
            );

            // Straight corners at top, rounded corners at bottom
            let rounding = egui::Rounding {
                nw: 0.0,
                ne: 0.0,
                sw: 16.0_f32.min(rect_height / 2.0),
                se: 16.0_f32.min(rect_height / 2.0),
            };

            ui.painter().rect_filled(
                rect,
                rounding,
                egui::Color32::from_black_alpha(240), // Darker notch
            );

            if self.is_listening_state.load(Ordering::Relaxed) {
                let num_stems = 15;
                let spacing = 3.0;
                let stem_width = (rect.width() - (spacing * (num_stems as f32 + 1.0)) - 20.0) / num_stems as f32;
                let mut x = rect.left() + 10.0 + spacing;

                for i in 0..num_stems {
                    let amplitude = self.stems[i];
                    // Constrain stem height using the current dynamic ui_height
                    let max_stem_h = (rect.height() - 8.0).max(2.0);
                    let stem_height = (amplitude * rect.height() * 0.8).clamp(2.0, max_stem_h);
                        
                        let t = i as f32 / num_stems as f32;
                        let color = if t < 0.33 {
                            egui::Color32::from_rgb(0, 255, 255)
                        } else if t < 0.66 {
                            egui::Color32::from_rgb(255, 0, 255)
                        } else {
                            egui::Color32::from_rgb(128, 0, 255)
                        };

                        let stem_rect = egui::Rect::from_center_size(
                            egui::pos2(x + stem_width / 2.0, rect.top() + rect.height() / 2.0),
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
                // Organic neon boundary contour trace
                let time_f = self.time as f32;
                let duration = 3.0; // Slower sweep looks more elegant
                let progress = (time_f % duration) / duration; // 0.0 to 1.0
                
                let r = 16.0_f32.min(rect.height());
                let l1 = (rect.height() - r).max(0.0);
                let l2 = r * std::f32::consts::PI / 2.0;
                let l3 = (rect.width() - 2.0 * r).max(0.0);
                let l4 = r * std::f32::consts::PI / 2.0;
                let l5 = (rect.height() - r).max(0.0);
                let total_l = l1 + l2 + l3 + l4 + l5;
                
                let get_point = |mut d: f32| -> egui::Pos2 {
                    if d <= l1 {
                        return egui::pos2(rect.left(), rect.top() + d);
                    }
                    d -= l1;
                    if d <= l2 {
                        let frac = d / l2;
                        let theta = std::f32::consts::PI - frac * (std::f32::consts::PI / 2.0);
                        let cx = rect.left() + r;
                        let cy = rect.bottom() - r;
                        return egui::pos2(cx + r * theta.cos(), cy + r * theta.sin());
                    }
                    d -= l2;
                    if d <= l3 {
                        return egui::pos2(rect.left() + r + d, rect.bottom());
                    }
                    d -= l3;
                    if d <= l4 {
                        let frac = d / l4;
                        let theta = std::f32::consts::PI / 2.0 - frac * (std::f32::consts::PI / 2.0);
                        let cx = rect.right() - r;
                        let cy = rect.bottom() - r;
                        return egui::pos2(cx + r * theta.cos(), cy + r * theta.sin());
                    }
                    d -= l4;
                    return egui::pos2(rect.right(), (rect.bottom() - r - d).max(rect.top()));
                };

                let tail_length = 120.0;
                let head_d = (total_l + tail_length) * progress;
                let segments = 45;
                let seg_len = tail_length / segments as f32;

                for j in 0..segments {
                    let d2 = head_d - j as f32 * seg_len;
                    let d1 = head_d - (j as f32 + 1.0) * seg_len;
                    
                    if d2 > 0.0 && d1 < total_l {
                        let p1 = get_point(d1.clamp(0.0, total_l));
                        let p2 = get_point(d2.clamp(0.0, total_l));
                        
                        let scale = 1.0 - (j as f32 / segments as f32);
                        let alpha = scale.powi(2); // Non-linear fade looks organic
                        let color = egui::Color32::from_rgba_premultiplied(
                            (0.0 * alpha) as u8,
                            (255.0 * alpha) as u8,
                            (255.0 * alpha) as u8,
                            (255.0 * alpha) as u8,
                        );
                        
                        let thickness = 3.0 * scale.max(0.1); // Tip is thick, tail is thin
                        ui.painter().line_segment([p1, p2], egui::Stroke::new(thickness, color));
                    }
                }
            }
        });
        
        ctx.request_repaint();
    }
}

