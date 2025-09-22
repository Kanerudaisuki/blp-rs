use crate::core::mipmap::Mipmap;
use crate::core::types::{SourceKind, TextureType, Version};
use crate::error::error::BlpError;

pub const MAX_MIPS: usize = 16;
pub const HEADER_SIZE: u64 = 156;

#[derive(Debug, Default)]
pub struct ImageBlp {
    #[allow(dead_code)]
    pub version: Version,
    pub texture_type: TextureType,
    pub compression: u8,
    pub alpha_bits: u32,
    pub alpha_type: u8,
    pub has_mips: u8,
    pub width: u32,
    pub height: u32,
    pub extra: u32,                      // meaningful only if version <= BLP1
    pub has_mipmaps: u32,                // meaningful only if version <= BLP1 or >= BLP2
    pub mipmap_offsets: [u32; MAX_MIPS], // valid if version >= BLP1
    pub mipmap_lengths: [u32; MAX_MIPS], // valid if version >= BLP1
    //
    pub mipmaps: Vec<Mipmap>,
    pub holes: usize,
    pub header_offset: usize,
    pub header_length: usize,
    //
    pub source: SourceKind,
}

impl ImageBlp {
    pub fn from_buf(buf: &[u8]) -> Result<Self, BlpError> {
        if buf.len() >= 3 && &buf[..3] == b"BLP" {
            Self::from_buf_blp(buf)
        } else {
            Self::from_buf_image(buf)
        }
    }

    pub fn decode(&mut self, buf: &[u8]) -> Result<(), BlpError> {
        match self.source {
            SourceKind::Blp => match self.texture_type {
                TextureType::DIRECT => self.decode_direct(buf),
                TextureType::JPEG => self.decode_jpeg(buf),
            },
            SourceKind::Image => self.decode_image(buf),
        }
    }
}
