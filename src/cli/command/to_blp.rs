use crate::err::error::BlpError;
use crate::image_blp::ImageBlp;
use crate::util::resolve_output_path::resolve_output_path;
use std::fs;
use std::path::{Path, PathBuf};

pub fn to_blp(input: &Path, output: Option<&PathBuf>) -> Result<(), BlpError> {
    input.try_exists()?;
    let data = fs::read(input)?;
    let img = ImageBlp::from_buf(&data)?;
    let out_path = resolve_output_path(input, output, "blp");
    img.export_blp(&out_path, 100, &[])?;
    println!("Saved BLP → {}", out_path.display());
    Ok(())
}
