use crate::image_blp::ImageBlp;
use std::error::Error;

impl ImageBlp {
    pub fn from_bytes(buf: &[u8]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        if buf.len() >= 3 && &buf[..3] == b"BLP" { Self::parse_blp(buf) } else { Self::decode_image(buf) }
    }
}
