use crate::err::blp_err::BlpErr;
use crate::image_blp::ImageBlp;

impl ImageBlp {
    pub fn from_buf(buf: &[u8]) -> Result<Self, BlpErr> {
        if buf.len() >= 3 && &buf[..3] == b"BLP" {
            Self::from_buf_blp(buf)
        } else {
            Self::from_buf_image(buf)
        }
    }
}
