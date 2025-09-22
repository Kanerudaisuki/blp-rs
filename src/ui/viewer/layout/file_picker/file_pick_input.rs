use crate::error::error::BlpError;
use crate::ext::path::ensure_readable::EnsureReadable;
use crate::core::image::ImageBlp;
use std::path::PathBuf;

pub enum FilePickInput {
    Path(PathBuf),
    Bytes(Vec<u8>),
}

impl FilePickInput {
    pub fn decode(self) -> Result<ImageBlp, BlpError> {
        match self {
            FilePickInput::Path(path) => {
                let data = path.as_path().read_all()?;
                ImageBlp::from_buf(&data)
            }
            FilePickInput::Bytes(data) => ImageBlp::from_buf(&data),
        }
    }
}
