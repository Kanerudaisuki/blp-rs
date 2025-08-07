use crate::ui::load_image::load_image;
use eframe::egui;
use egui::{ColorImage, TextureHandle};
use std::path::PathBuf;

pub struct ViewerApp {
    file: Option<PathBuf>,
    image: Option<ColorImage>,
    texture: Option<TextureHandle>,
}

impl ViewerApp {
    pub fn new(file: Option<PathBuf>) -> Self {
        let mut app = Self { file: file.clone(), image: None, texture: None };

        if let Some(path) = &file {
            if let Ok(img) = load_image(path) {
                app.image = Some(img);
            }
        }
        app
    }
}

impl eframe::App for ViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("🖼 BLP Viewer");

            if ui.button("📂 Open file...").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    self.file = Some(path.clone());
                    if let Ok(img) = load_image(&path) {
                        self.image = Some(img);
                        self.texture = None; // сброс texture, пересоздастся
                    }
                }
            }

            if let Some(path) = &self.file {
                ui.label(format!("📄 File: {}", path.display()));
            }

            if let Some(img) = &self.image {
                if self.texture.is_none() {
                    self.texture = Some(ctx.load_texture("image", img.clone(), egui::TextureOptions::default()));
                }

                if let Some(texture) = &self.texture {
                    ui.image(texture);
                }
            }
        });
    }
}
