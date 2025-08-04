#![allow(dead_code)]

pub mod header;
mod image_blp;
pub mod read;
mod texture_type;
pub mod version;
mod mipmap;

use crate::image_blp::ImageBlp;
use image::{DynamicImage, ImageFormat};
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::Path;

pub fn convert_and_save_pngs(path: &Path) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let buf = fs::read(path)?;
    let image = ImageBlp::from_bytes(&buf)?;

    let stem = path
        .file_stem()
        .ok_or("Invalid file name")?
        .to_string_lossy();

    for mip in &image.mipmaps {
        if let Some(rgba) = &mip.image {
            let filename = format!("{stem}{}x{}.png", mip.width, mip.height);
            let output_path = path.with_file_name(filename);
            let file = File::create(&output_path)?;
            let mut writer = BufWriter::new(file);

            DynamicImage::ImageRgba8(rgba.clone()).write_to(&mut writer, ImageFormat::Png)?;
        }
    }

    Ok(())
}

fn main() {
    //let path = Path::new("test-data/scan/BLP1_tt1_c0_ab0_at0_m0_512x512.blp");
    let path = Path::new("test-data/Arthas.blp");


    if let Err(e) = convert_and_save_pngs(path) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
