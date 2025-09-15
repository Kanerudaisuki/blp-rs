use crate::err::blp_err::BlpErr;
use crate::ext::path::ensure_readable::EnsureReadable;
use crate::image_blp::ImageBlp;
use std::path::PathBuf;

pub enum DecodeInput {
    Path(PathBuf),
    Bytes(Vec<u8>),
}

impl DecodeInput {
    pub fn decode(self) -> Result<ImageBlp, BlpErr> {
        match self {
            DecodeInput::Path(path) => {
                let data = path.as_path().read_all()?;
                ImageBlp::from_buf(&data)
            }
            DecodeInput::Bytes(data) => ImageBlp::from_buf(&data),
        }
    }
}
