pub(crate) use crate::header::Header;
use crate::mipmap::Mipmap;
pub const MAX_MIPS: usize = 16;

#[derive(Debug)]
pub struct ImageBlp {
    pub header: Header,
    pub mipmaps: Vec<Mipmap>,
    pub holes: usize,
}
