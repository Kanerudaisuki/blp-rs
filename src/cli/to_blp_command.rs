use crate::cli::ensure_input_exists::ensure_input_exists;
use crate::cli::resolve_output_path::resolve_output_path;
use crate::export::blp::export_blp;
use crate::image_blp::ImageBlp;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

pub fn to_blp_command(input: &Path, output: Option<&PathBuf>) -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("Converting to BLP");
    ensure_input_exists(input)?;
    let data = fs::read(input)?;
    let img = ImageBlp::from_bytes(&data)?; // ест BLP и любые растры
    let out_path = resolve_output_path(input, output, "blp");
    export_blp(&img, &out_path, 100)?; // дефолтное качество
    println!("Saved BLP → {}", out_path.display());
    Ok(())
}
