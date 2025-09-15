use crate::err::blp_err::BlpErr;
use crate::export::export_png::export_png;
use crate::image_blp::ImageBlp;
use crate::util::resolve_output_path::resolve_output_path;
use std::fs;
use std::path::{Path, PathBuf};

pub fn to_png(input: &Path, output: Option<&PathBuf>) -> Result<(), BlpErr> {
    input.try_exists()?;
    let data = fs::read(input)?;
    let img = ImageBlp::from_buf(&data).map_err(|e| e.ctx("blp.decode-failed"))?;

    let out_path = resolve_output_path(input, output, "png");
    export_png(&img, &out_path)?;
    println!("Saved PNG → {}", out_path.display());
    Ok(())
}
