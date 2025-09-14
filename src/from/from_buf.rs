use crate::err::app_err::AppErr;
use crate::image_blp::ImageBlp;

impl ImageBlp {
    pub fn from_buf(buf: &[u8]) -> Result<Self, AppErr> {
        if buf.len() >= 3 && &buf[..3] == b"BLP" {
            Self::from_buf_blp(buf)
        } else {
            Self::from_buf_image(buf)
        }
    }
}
