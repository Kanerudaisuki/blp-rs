use crate::encode::blp::options::EncoderOptions;
use crate::err::error::BlpError;
use crate::image_blp::ImageBlp;
use std::fs;
use std::path::Path;

pub fn export_blp(img: &ImageBlp, out_path: &Path, quality: u8) -> Result<(), BlpError> {
    if let Some(parent) = out_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    // Полный пайплайн: контекст содержит всё (bytes, mips, счётчики, и т.д.)
    let ctx = img.encode_blp(EncoderOptions { quality, mip_visible: &[] })?;

    // Сохраняем готовый контейнер
    fs::write(out_path, &ctx.bytes)?;
    Ok(())
}
