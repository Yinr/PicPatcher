use std::path::{Path, PathBuf};

use egui::{ColorImage, Key, Sense, TextureHandle, TextureOptions};
use image::GenericImageView;
use picpatcher_core::{Config, DEFAULT_EXTENSIONS};

use crate::i18n::Texts;

pub struct LoadedImage {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub texture: TextureHandle,
}

pub struct EditorState {
    pub base: Option<LoadedImage>,
    pub overlay: Option<LoadedImage>,
    /// Overlay position in BASE image pixel coordinates (top-left).
    pub x: i64,
    pub y: i64,
    pub config_path: Option<PathBuf>,
    pub status: String,
    /// Whether overlay path was loaded from an existing config (for relative paths).
    pub overlay_was_relative: bool,
    /// Multiplier on top of fit-to-window scale. 1.0 means fit image to viewport.
    pub zoom: f32,
    last_fit_scale: f32,
    drag_remainder: egui::Vec2,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            base: None,
            overlay: None,
            x: 0,
            y: 0,
            config_path: None,
            status: String::new(),
            overlay_was_relative: false,
            zoom: 1.0,
            last_fit_scale: 1.0,
            drag_remainder: egui::Vec2::ZERO,
        }
    }
}

fn load_to_texture(ctx: &egui::Context, path: &Path, name: &str) -> anyhow::Result<LoadedImage> {
    let img = image::open(path)?;
    let (w, h) = img.dimensions();
    let rgba = img.to_rgba8();
    let color = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], rgba.as_raw());
    let texture = ctx.load_texture(name, color, TextureOptions::LINEAR);
    Ok(LoadedImage {
        path: path.to_path_buf(),
        width: w,
        height: h,
        texture,
    })
}

impl EditorState {
    pub fn ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, t: &Texts) {
        self.handle_keyboard_nudge(ctx);

        ui.horizontal(|ui| {
            if ui.button(t.open_base_image).clicked() {
                if let Some(p) = rfd::FileDialog::new()
                    .add_filter(t.images, DEFAULT_EXTENSIONS)
                    .pick_file()
                {
                    match load_to_texture(ctx, &p, "base") {
                        Ok(img) => {
                            self.base = Some(img);
                            self.status.clear();
                        }
                        Err(e) => self.status = format!("{}: {e:#}", t.base_load_failed),
                    }
                }
            }
            if ui.button(t.open_overlay).clicked() {
                if let Some(p) = rfd::FileDialog::new()
                    .add_filter(t.png_recommended, &["png"])
                    .add_filter(t.images, DEFAULT_EXTENSIONS)
                    .pick_file()
                {
                    match load_to_texture(ctx, &p, "overlay") {
                        Ok(img) => {
                            self.overlay = Some(img);
                            self.overlay_was_relative = false;
                            self.status.clear();
                        }
                        Err(e) => self.status = format!("{}: {e:#}", t.overlay_load_failed),
                    }
                }
            }
            ui.separator();
            if ui.button(t.load_config).clicked() {
                if let Some(p) = rfd::FileDialog::new()
                    .add_filter("JSON", &["json"])
                    .pick_file()
                {
                    self.load_config(ctx, &p, t);
                }
            }
            if ui.button(t.save_config).clicked() {
                self.save_config(false, t);
            }
            if ui.button(t.save_config_as).clicked() {
                self.save_config(true, t);
            }
        });

        ui.separator();

        egui::SidePanel::right("editor_side")
            .resizable(true)
            .default_width(280.0)
            .show_inside(ui, |ui| {
                ui.heading(t.parameters);
                ui.add_space(4.0);
                ui.label(t.canvas_zoom);
                ui.horizontal(|ui| {
                    if ui.button(t.fit).clicked() {
                        self.zoom = 1.0;
                    }
                    if ui.button("100%").clicked() {
                        self.zoom = (1.0 / self.last_fit_scale).clamp(0.25, 16.0);
                    }
                    ui.label(format!("{:.0}%", self.effective_zoom_percent()));
                });
                ui.add(
                    egui::Slider::new(&mut self.zoom, 0.25..=16.0)
                        .logarithmic(true)
                        .show_value(false),
                );
                ui.small(t.drag_hint);
                ui.add_space(8.0);
                ui.label(t.overlay_x);
                ui.add(egui::DragValue::new(&mut self.x).speed(1.0));
                ui.label(t.overlay_y);
                ui.add(egui::DragValue::new(&mut self.y).speed(1.0));
                ui.add_space(8.0);
                if let Some(b) = &self.base {
                    ui.label(format!("{}: {} x {}", t.base, b.width, b.height));
                }
                if let Some(o) = &self.overlay {
                    ui.label(format!("{}: {} x {}", t.overlay, o.width, o.height));
                    ui.label(format!("{}: {}", t.path, o.path.display()));
                }
                ui.add_space(8.0);
                if !self.status.is_empty() {
                    ui.colored_label(egui::Color32::LIGHT_RED, &self.status);
                }
                if let Some(p) = &self.config_path {
                    ui.label(format!("{}: {}", t.config, p.display()));
                }
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.canvas(ui, t);
        });
    }

    fn canvas(&mut self, ui: &mut egui::Ui, t: &Texts) {
        let Some(base) = &self.base else {
            ui.centered_and_justified(|ui| {
                ui.label(t.open_base_to_start);
            });
            return;
        };

        let avail = ui.available_size();
        let bw = base.width as f32;
        let bh = base.height as f32;
        let fit_scale = (avail.x / bw).min(avail.y / bh).min(1.0).max(0.01);
        self.last_fit_scale = fit_scale;
        let scale = (fit_scale * self.zoom).clamp(0.01, 32.0);
        let disp = egui::vec2(bw * scale, bh * scale);

        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                self.canvas_contents(ui, scale, disp, t);
            });
    }

    fn canvas_contents(&mut self, ui: &mut egui::Ui, scale: f32, disp: egui::Vec2, t: &Texts) {
        let Some(base) = &self.base else { return };

        let (rect, _resp) = ui.allocate_exact_size(disp, Sense::hover());
        let painter = ui.painter_at(rect);

        // Draw base
        painter.image(
            base.texture.id(),
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );

        // Draw overlay (if any) as an interactive draggable item
        if let Some(ov) = &self.overlay {
            let ow = ov.width as f32 * scale;
            let oh = ov.height as f32 * scale;
            let ox = rect.min.x + self.x as f32 * scale;
            let oy = rect.min.y + self.y as f32 * scale;
            let ov_rect = egui::Rect::from_min_size(egui::pos2(ox, oy), egui::vec2(ow, oh));
            let id = ui.make_persistent_id("overlay_drag");
            let resp = ui.interact(ov_rect, id, Sense::click_and_drag());
            painter.image(
                ov.texture.id(),
                ov_rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
            // Outline
            let stroke_color = if resp.hovered() || resp.dragged() {
                egui::Color32::YELLOW
            } else {
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 160)
            };
            painter.rect_stroke(ov_rect, 0.0, egui::Stroke::new(1.5, stroke_color));

            if resp.dragged() {
                let d = ui.input(|i| i.pointer.delta()) / scale + self.drag_remainder;
                let move_x = d.x.trunc() as i64;
                let move_y = d.y.trunc() as i64;
                self.drag_remainder = egui::vec2(d.x - move_x as f32, d.y - move_y as f32);
                self.x += move_x;
                self.y += move_y;
            } else {
                self.drag_remainder = egui::Vec2::ZERO;
            }

            self.clamp_overlay_position();
        } else {
            ui.allocate_ui_at_rect(rect, |ui| {
                ui.label(t.open_overlay_to_position);
            });
        }
    }

    fn handle_keyboard_nudge(&mut self, ctx: &egui::Context) {
        if self.base.is_none() || self.overlay.is_none() || ctx.wants_keyboard_input() {
            return;
        }

        let (dx, dy) = ctx.input(|i| {
            let step = if i.modifiers.shift { 10 } else { 1 };
            let mut dx = 0;
            let mut dy = 0;
            if i.key_pressed(Key::ArrowLeft) {
                dx -= step;
            }
            if i.key_pressed(Key::ArrowRight) {
                dx += step;
            }
            if i.key_pressed(Key::ArrowUp) {
                dy -= step;
            }
            if i.key_pressed(Key::ArrowDown) {
                dy += step;
            }
            (dx, dy)
        });

        if dx != 0 || dy != 0 {
            self.x += dx;
            self.y += dy;
            self.clamp_overlay_position();
            ctx.request_repaint();
        }
    }

    fn effective_zoom_percent(&self) -> f32 {
        self.last_fit_scale * self.zoom * 100.0
    }

    fn clamp_overlay_position(&mut self) {
        let (Some(base), Some(overlay)) = (&self.base, &self.overlay) else {
            return;
        };
        // Clamp so overlay is at least partially in image.
        let max_x = base.width as i64 - 1;
        let max_y = base.height as i64 - 1;
        let min_x = -(overlay.width as i64 - 1);
        let min_y = -(overlay.height as i64 - 1);
        self.x = self.x.clamp(min_x, max_x);
        self.y = self.y.clamp(min_y, max_y);
    }

    fn load_config(&mut self, ctx: &egui::Context, path: &Path, t: &Texts) {
        match Config::load(path) {
            Ok(cfg) => {
                self.x = cfg.x;
                self.y = cfg.y;
                self.config_path = Some(path.to_path_buf());
                let overlay_abs = cfg.resolve_overlay(path);
                match load_to_texture(ctx, &overlay_abs, "overlay") {
                    Ok(img) => {
                        self.overlay = Some(img);
                        self.overlay_was_relative = !cfg.overlay.is_absolute();
                        self.status = format!("{} {}", t.loaded, path.display());
                    }
                    Err(e) => self.status = format!("{}: {e:#}", t.overlay_load_failed),
                }
            }
            Err(e) => self.status = format!("{}: {e:#}", t.load_failed),
        }
    }

    fn save_config(&mut self, as_new: bool, t: &Texts) {
        let Some(overlay) = &self.overlay else {
            self.status = t.no_overlay_loaded.into();
            return;
        };
        let target = if as_new || self.config_path.is_none() {
            rfd::FileDialog::new()
                .add_filter("JSON", &["json"])
                .set_file_name("picpatcher.json")
                .save_file()
        } else {
            self.config_path.clone()
        };
        let Some(target) = target else { return };

        // Try to make overlay path relative to config directory.
        let overlay_path = if let Some(parent) = target.parent() {
            pathdiff_relative(&overlay.path, parent).unwrap_or_else(|| overlay.path.clone())
        } else {
            overlay.path.clone()
        };

        let cfg = Config {
            version: picpatcher_core::CONFIG_VERSION,
            overlay: overlay_path,
            x: self.x,
            y: self.y,
        };
        match cfg.save(&target) {
            Ok(()) => {
                self.config_path = Some(target.clone());
                self.status = format!("{} {}", t.saved, target.display());
            }
            Err(e) => self.status = format!("{}: {e:#}", t.save_failed),
        }
    }
}

/// Compute a relative path from `base` to `target`, if both are absolute and share a prefix.
/// Falls back to None to let caller use the absolute path.
fn pathdiff_relative(target: &Path, base: &Path) -> Option<PathBuf> {
    let t = target.canonicalize().ok()?;
    let b = base.canonicalize().ok()?;
    // Simple component-wise diff (works on same drive on Windows).
    use std::path::Component;
    let mut tc: Vec<Component> = t.components().collect();
    let mut bc: Vec<Component> = b.components().collect();
    // Drive/root must match
    let mut common = 0usize;
    while common < tc.len() && common < bc.len() && tc[common] == bc[common] {
        common += 1;
    }
    if common == 0 {
        return None;
    }
    let ups = bc.split_off(common).len();
    let rest: PathBuf = tc.split_off(common).iter().collect();
    let mut out = PathBuf::new();
    for _ in 0..ups {
        out.push("..");
    }
    out.push(rest);
    Some(out)
}
