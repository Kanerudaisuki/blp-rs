use crate::ui::load_image::load_image;
use eframe::egui;
use egui::TextureHandle;
use std::path::PathBuf;

pub struct ViewerApp {
    file: Option<PathBuf>,
    texture: Option<TextureHandle>,
    needs_reload: bool,
}

impl ViewerApp {
    pub fn new(path: Option<PathBuf>, ctx: &egui::Context) -> Self {
        let mut app = Self { file: None, texture: None, needs_reload: false };

        if let Some(path) = path {
            app.set_file(ctx, path);
        }

        app
    }

    pub fn set_file(&mut self, ctx: &egui::Context, path: PathBuf) {
        match load_image(&path) {
            Ok(image) => {
                self.texture = Some(ctx.load_texture("blp-image", image, egui::TextureOptions::default()));
                self.file = Some(path);
                self.needs_reload = false;
            }
            Err(err) => {
                eprintln!("❌ Failed to load image: {err}");
                self.texture = None;
                self.file = None;
                self.needs_reload = false;
            }
        }
    }
}

impl eframe::App for ViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.needs_reload {
            if let Some(path) = self.file.clone() {
                self.set_file(ctx, path);
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Frame::default()
                .fill(egui::Color32::TRANSPARENT)
                .show(ui, |ui| {
                    ui.heading("🖼 BLP Viewer");

                    if ui.button("📂 Open file...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                            self.file = Some(path);
                            self.needs_reload = true;
                        }
                    }

                    if let Some(path) = &self.file {
                        ui.label(format!("📄 File: {}", path.display()));
                    }

                    if let Some(texture) = &self.texture {
                        ui.image(texture);
                    }
                });
        });
    }
}
