use eframe::egui;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::runtime::Runtime;
use tokio::sync::mpsc as tokio_mpsc;
use std::sync::mpsc as std_mpsc;

pub struct FlowApp {
    #[allow(dead_code)]
    _rt: Arc<Runtime>,
    #[allow(dead_code)]
    is_listening_state: Arc<AtomicBool>,
    is_processing_state: Arc<AtomicBool>,
    current_amplitude: Arc<std::sync::atomic::AtomicU32>,
    #[allow(dead_code)]
    audio_tx: (),
    #[allow(dead_code)]
    rx_app_event: std_mpsc::Receiver<()>,
    tx_app_event: tokio_mpsc::Sender<crate::automation::AppEvent>,
    menu_open: bool,
    menu_pos: egui::Pos2,
    frames_since_menu_open: u64,
    stems: Vec<f32>,
    target_stems: Vec<f32>,
    time: f64,
    has_positioned: bool,
    ui_width: f32,
    ui_height: f32,
}

impl FlowApp {
    pub fn new(
        _cc: &eframe::CreationContext<'_>,
        _rt: Arc<Runtime>,
        is_listening_state: Arc<AtomicBool>,
        is_processing_state: Arc<AtomicBool>,
        current_amplitude: Arc<std::sync::atomic::AtomicU32>,
        audio_tx: (),
        rx_app_event: std_mpsc::Receiver<()>,
        tx_app_event: tokio_mpsc::Sender<crate::automation::AppEvent>,
    ) -> Self {
        Self {
            _rt,
            is_listening_state,
            is_processing_state,
            current_amplitude,
            audio_tx,
            rx_app_event,
            tx_app_event,
            menu_open: false,
            menu_pos: egui::pos2(0.0, 0.0),
            frames_since_menu_open: 0,
            stems: vec![0.0; 35],
            target_stems: vec![0.0; 35],
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
        use std::io::Write;
        // Startup log so we know the directory
        if self.frames_since_menu_open == 0 && self.time == 0.0 {
            if let Ok(mut f) = std::fs::OpenOptions::new().append(true).create(true).open("flow.log") {
                let _ = writeln!(f, "DEBUG: UI Update loop started.");
            }
        }

        // Pill logic
        if !self.has_positioned {
            if let Some(monitor_size) = ctx.input(|i| i.viewport().monitor_size) {
                let window_width = 300.0;
                let pos_x = (monitor_size.x - window_width) / 2.0;
                let pos_y = 0.0; // Touch the very top edge
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

        for i in 0..self.stems.len() {
            if self.is_listening_state.load(Ordering::Relaxed) {
                // Symmetrical envelope: center stems are taller, outer stems are shorter
                let center_dist = (i as f32 - (self.stems.len() as f32 - 1.0) / 2.0).abs();
                let envelope = 1.0 - (center_dist / (self.stems.len() as f32 / 2.0)).powi(2); // Parabolic curve
                
                let phase = (self.time as f32 * 4.0 + i as f32 * 0.4).sin() * 0.5 + 0.5; // 0..1
                let target = (mic_amp * envelope.max(0.0) * phase) + 0.08;
                self.target_stems[i] = target;
            } else {
                // Idle animation
                let idle_sine = (self.time as f32 * 1.5 + i as f32 * 0.2).sin().abs() * 0.1;
                self.target_stems[i] = idle_sine + 0.1;
            }
            
            // Spring-like fluid interpolation
            self.stems[i] += (self.target_stems[i] - self.stems[i]) * dt * 20.0;
        }

        let is_listening = self.is_listening_state.load(Ordering::Relaxed);
        let is_processing = self.is_processing_state.load(Ordering::Relaxed);

        let target_width = if is_listening { 320.0 } else if is_processing { 280.0 } else { 240.0 };
        let target_height = if is_listening { 70.0 } else if is_processing { 32.0 } else { 16.0 };
        
        // Slower, smoother transition
        self.ui_width += (target_width - self.ui_width) * (dt * 8.0).min(1.0);
        self.ui_height += (target_height - self.ui_height) * (dt * 8.0).min(1.0);

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

            let response = ui.allocate_rect(rect, egui::Sense::click());
            if response.clicked() {
                let _ = self.tx_app_event.try_send(crate::automation::AppEvent::ToggleListening);
            }

            // Straight corners at top, rounded corners at bottom
            // Force the radius to always be perfect semicircles for the dynamic island look
            let current_radius = rect_height / 2.0;
            let rounding = egui::Rounding {
                nw: 0.0,
                ne: 0.0,
                sw: current_radius,
                se: current_radius,
            };

            ui.painter().rect_filled(
                rect,
                rounding,
                egui::Color32::from_black_alpha(180), // Semi-transparent notch
            );

            if self.is_listening_state.load(Ordering::Relaxed) {
                let num_stems = self.stems.len();
                let spacing = 2.0;
                let padding = 20.0;
                let stem_width = (rect.width() - (spacing * (num_stems as f32 - 1.0)) - padding * 2.0) / num_stems as f32;
                let mut x = rect.left() + padding;

                for i in 0..num_stems {
                    let amplitude = self.stems[i];
                    // Constrain stem height
                    let max_stem_h = (rect.height() - 16.0).max(2.0);
                    let stem_height = (amplitude * rect.height() * 0.8).clamp(2.0, max_stem_h);
                        
                        let t = i as f32 / (num_stems as f32 - 1.0); // 0.0 to 1.0
                        
                        // Premium fluid gradient (Cyan -> Deep Blue -> Pink/Purple)
                        let r = (255.0 * t.powi(2)) as u8;
                        let g = (255.0 * (1.0 - t).powi(2)) as u8;
                        let b = 255;
                        
                        let color = egui::Color32::from_rgb(r, g, b);
                        let glow = egui::Color32::from_rgba_premultiplied(
                            (r as f32 * 0.4) as u8,
                            (g as f32 * 0.4) as u8,
                            (b as f32 * 0.4) as u8,
                            100
                        );

                        let stem_rect = egui::Rect::from_center_size(
                            egui::pos2(x + stem_width / 2.0, rect.top() + rect.height() / 2.0),
                            egui::vec2(stem_width, stem_height),
                        );

                        // Glow behind the stem
                        ui.painter().rect_filled(
                            stem_rect.expand(2.0),
                            (stem_width / 2.0) + 2.0,
                            glow,
                        );

                        // Core
                        ui.painter().rect_filled(
                            stem_rect,
                            stem_width / 2.0,
                            color,
                        );

                    x += stem_width + spacing;
                }
            } else if is_processing {
                // Processing loader: scanning gradient or rotating ball inside the pill
                let time_f = self.time as f32;
                let cycle = (time_f * 2.0) % (std::f32::consts::PI * 2.0);
                let x_offset = cycle.sin() * (rect.width() / 2.0 - 24.0);
                
                let cx = rect.center().x + x_offset;
                let cy = rect.center().y;
                let loader_radius = 6.0;

                // Color pulse
                let pulse = (time_f * 5.0).sin() * 0.5 + 0.5;
                let color = egui::Color32::from_rgba_premultiplied(
                    (100.0 + 155.0 * pulse) as u8,
                    (200.0 + 55.0 * pulse) as u8,
                    255,
                    255
                );
                
                // Draw a simple, elegant glowing orb moving side to side
                ui.painter().circle_filled(
                    egui::pos2(cx, cy),
                    loader_radius,
                    color
                );
                ui.painter().circle_filled(
                    egui::pos2(cx, cy),
                    loader_radius + 4.0,
                    egui::Color32::from_rgba_premultiplied(
                        (100.0 * 0.4) as u8,
                        (200.0 * 0.4) as u8,
                        (255.0 * 0.4) as u8,
                        100
                    )
                );
            } else {
                // Organic illumination passing from left to right along the entire bottom border
                let time_f = self.time as f32;
                let duration = 6.0; // Slow, calm sweep
                let progress = (time_f % duration) / duration; // 0.0 to 1.0
                
                let max_radius: f32 = 32.0;
                let r = max_radius.min(rect.height() / 2.0);
                let l1 = (rect.height() - r).max(0.0);
                let l2 = r * std::f32::consts::PI / 2.0;
                let l3 = (rect.width() - 2.0 * r).max(0.0);
                let l4 = r * std::f32::consts::PI / 2.0;
                let l5 = (rect.height() - r).max(0.0);
                let total_l = l1 + l2 + l3 + l4 + l5;
                
                let get_point = |mut d: f32| -> egui::Pos2 {
                    if d <= 0.0 {
                        return egui::pos2(rect.left(), rect.top());
                    }
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
                    if d <= l5 {
                        return egui::pos2(rect.right(), rect.bottom() - r - d);
                    }
                    return egui::pos2(rect.right(), rect.top());
                };

                let beam_width = 80.0; // Shorter beam
                let start_d = -beam_width;
                let end_d = total_l + beam_width;
                let center_d = start_d + (end_d - start_d) * progress;
                
                let segments = 60;
                let seg_len = beam_width / segments as f32;
                
                for j in 0..=segments {
                    let offset = (j as f32 - (segments as f32 / 2.0)) * seg_len;
                    let pd = center_d + offset;
                    
                    if pd >= 0.0 && pd <= total_l {
                        let dist = offset.abs();
                        // Cosine wave for perfectly smooth, organic fade
                        let normalized_dist = (dist / (beam_width / 2.0)).clamp(0.0, 1.0);
                        let alpha = (std::f32::consts::PI * normalized_dist / 2.0).cos();
                        
                        if alpha > 0.01 {
                            let alpha_sq = alpha * alpha * alpha; // Cubic curve for soft tail and intense core
                            
                            let p1 = get_point(pd);
                            let p2 = get_point((pd + seg_len).min(total_l));
                            
                            // Outer soft glow (deep cyan/blue)
                            let glow_color = egui::Color32::from_rgba_premultiplied(
                                0,
                                (150.0 * alpha_sq * 0.5) as u8,
                                (255.0 * alpha_sq * 0.5) as u8,
                                (255.0 * alpha_sq * 0.5) as u8,
                            );
                            ui.painter().line_segment([p1, p2], egui::Stroke::new(3.0, glow_color));

                            // Inner intense core (bright white/cyan)
                            let core_color = egui::Color32::from_rgba_premultiplied(
                                (200.0 * alpha_sq * 0.9) as u8,
                                (255.0 * alpha_sq * 0.9) as u8,
                                (255.0 * alpha_sq * 0.9) as u8,
                                (255.0 * alpha_sq * 0.9) as u8,
                            );
                            ui.painter().line_segment([p1, p2], egui::Stroke::new(1.0, core_color));
                        }
                    }
                }
            }
        });
        
        ctx.request_repaint();
    }
}

