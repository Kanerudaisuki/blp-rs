use eframe::egui;
use egui::{ColorImage, TextureHandle};
use image::DynamicImage;
use image::ImageReader;
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

fn load_image(path: &PathBuf) -> Result<ColorImage, image::ImageError> {
    let img = ImageReader::open(path)?.decode()?;
    Ok(dynamic_image_to_color_image(&img))
}

fn dynamic_image_to_color_image(img: &DynamicImage) -> ColorImage {
    let rgba = img.to_rgba8();
    let [w, h] = [rgba.width() as usize, rgba.height() as usize];

    let pixels = rgba
        .chunks(4)
        .map(|p| egui::Color32::from_rgba_premultiplied(p[0], p[1], p[2], p[3]))
        .collect();

    ColorImage { size: [w, h], pixels, source_size: egui::vec2(w as f32, h as f32) }
}
