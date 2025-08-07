use crate::ui::dynamic_image_to_color_image::dynamic_image_to_color_image;
use eframe::egui::ColorImage;
use image::ImageReader;
use std::path::PathBuf;

pub fn load_image(path: &PathBuf) -> Result<ColorImage, image::ImageError> {
    let img = ImageReader::open(path)?.decode()?;
    Ok(dynamic_image_to_color_image(&img))
}
