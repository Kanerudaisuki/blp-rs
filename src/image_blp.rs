pub(crate) use crate::header::Header;
use crate::mipmap::Mipmap;

#[derive(Debug)]
pub struct ImageBlp {
    pub header: Header,
    pub mipmaps: Vec<Mipmap>,
    pub holes: usize,
}
