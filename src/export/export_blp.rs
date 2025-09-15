use crate::err::blp_err::BlpErr;
use crate::image_blp::ImageBlp;
use std::fs;
use std::path::Path;

pub fn export_blp(img: &ImageBlp, out_path: &Path, quality: u8) -> Result<(), BlpErr> {
    if let Some(parent) = out_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let bytes = img.encode_blp(quality)?;
    fs::write(out_path, &bytes)?;
    Ok(())
}
