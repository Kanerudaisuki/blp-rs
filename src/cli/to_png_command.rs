use crate::cli::ensure_input_exists::ensure_input_exists;
use crate::cli::resolve_output_path::resolve_output_path;
use crate::export::png::export_png;
use crate::image_blp::ImageBlp;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

pub fn to_png_command(input: &Path, output: Option<&PathBuf>) -> Result<(), Box<dyn Error + Send + Sync>> {
    ensure_input_exists(input)?;
    let data = fs::read(input)?;
    let img = ImageBlp::from_bytes(&data).map_err(|e| format!("BLP decode failed: {e}"))?;
    let out_path = resolve_output_path(input, output, "png");
    export_png(&img, &out_path)?;
    println!("Saved PNG → {}", out_path.display());
    Ok(())
}
