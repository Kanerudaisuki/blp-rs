#![allow(dead_code)]

pub mod header;
pub mod read;
mod texture_type;
pub mod version;

use crate::read::convert_blp_to_rgba_image;
use image::{ImageFormat, RgbaImage};
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

pub fn convert_and_save_png(path: &Path) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let buf = std::fs::read(path)?;
    let rgba: RgbaImage = convert_blp_to_rgba_image(&buf)?;

    let output_path = path.with_extension("png");
    let file = File::create(&output_path)?;
    let mut writer = BufWriter::new(file);

    image::DynamicImage::ImageRgba8(rgba).write_to(&mut writer, ImageFormat::Png)?;

    Ok(())
}

fn main() {
    let path = Path::new("asset/Arthas.blp");
    if let Err(e) = convert_and_save_png(path) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
