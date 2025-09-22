pub(crate) use crate::header::Header;
use crate::mipmap::Mipmap;
pub const MAX_MIPS: usize = 16;

#[derive(Debug, Default)]
pub struct ImageBlp {
    pub header: Header,
    pub mipmaps: Vec<Mipmap>,
    pub holes: usize,
    pub jpeg_header_offset: usize,
    pub jpeg_header_length: usize,
}
