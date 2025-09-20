use crate::encode::blp::options::EncoderOptions;
use crate::err::error::BlpError;
use crate::image_blp::ImageBlp;

pub fn export_blp(img: &ImageBlp, out_path: &std::path::Path, quality: u8) -> Result<(), BlpError> {
    use std::fs;
    if let Some(parent) = out_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let report = img.encode_blp(EncoderOptions { quality, mip_visible: &[] })?;
    println!("{report}");
    fs::write(out_path, &report.bytes)?;
    Ok(())
}
