use crate::cli::ensure_input_exists::ensure_input_exists;
use crate::cli::resolve_output_path::resolve_output_path;
use crate::image_blp::ImageBlp;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

/// Конвертация BLP → PNG
pub fn to_png_command(input: &Path, output: Option<&PathBuf>) -> Result<(), Box<dyn Error + Send + Sync>> {
    ensure_input_exists(input)?;

    let data = fs::read(input)?;
    let out_path = resolve_output_path(input, output, "png");

    if let Some(parent) = out_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let blp = ImageBlp::from_bytes(&data).map_err(|e| format!("BLP decode failed: {e}"))?;
    let mip0 = blp
        .mipmaps
        .get(0)
        .and_then(|m| m.image.as_ref())
        .ok_or("BLP file has no base mip image (mip0)")?;

    image::DynamicImage::ImageRgba8(mip0.clone()).save(&out_path)?;
    println!("Saved PNG → {}", out_path.display());

    Ok(())
}
