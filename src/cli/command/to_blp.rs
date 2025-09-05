use crate::export::export_blp::export_blp;
use crate::image_blp::ImageBlp;
use crate::util::resolve_output_path::resolve_output_path;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
pub fn to_blp(input: &Path, output: Option<&PathBuf>) -> Result<(), Box<dyn Error + Send + Sync>> {
    input.try_exists()?;
    let data = fs::read(input)?;
    let img = ImageBlp::from_buf(&data)?;
    let out_path = resolve_output_path(input, output, "blp");
    export_blp(&img, &out_path, 100)?;
    println!("Saved BLP → {}", out_path.display());
    Ok(())
}
