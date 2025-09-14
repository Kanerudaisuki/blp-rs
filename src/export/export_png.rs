use crate::err::app_err::AppErr;
use crate::image_blp::ImageBlp;
use image::DynamicImage;
use std::fs;
use std::path::Path;

pub fn export_png(img: &ImageBlp, out_path: &Path) -> Result<(), AppErr> {
    if let Some(parent) = out_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let mip0 = img
        .mipmaps
        .get(0)
        .and_then(|m| m.image.as_ref())
        .ok_or_else(|| AppErr::new("error-export_png").with_arg("msg", "No base mip (mip0) RGBA image"))?;
    DynamicImage::ImageRgba8(mip0.clone()).save(out_path)?;
    Ok(())
}
