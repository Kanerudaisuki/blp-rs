use eframe::egui;
use eframe::egui::ColorImage;
use image::DynamicImage;

pub fn dynamic_image_to_color_image(img: &DynamicImage) -> ColorImage {
    let rgba = img.to_rgba8();
    let [w, h] = [rgba.width() as usize, rgba.height() as usize];

    let pixels = rgba
        .chunks(4)
        .map(|p| egui::Color32::from_rgba_premultiplied(p[0], p[1], p[2], p[3]))
        .collect();

    ColorImage { size: [w, h], pixels, source_size: egui::vec2(w as f32, h as f32) }
}
