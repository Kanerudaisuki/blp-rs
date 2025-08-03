use crate::texture_type::TextureType;
use crate::version::Version;
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use std::error::Error;
use std::io::Cursor;

#[derive(Debug)]
pub struct Header {
    pub version: Version,
    pub texture_type: TextureType,
    pub compression: u8,
    pub alpha_bits: u32,
    pub alpha_type: u8,
    pub has_mips: u8,
    pub width: u32,
    pub height: u32,
    pub extra: u32,                // meaningful only if version <= BLP1
    pub has_mipmaps: u32,          // meaningful only if version <= BLP1 or >= BLP2
    pub mipmap_offsets: [u32; 16], // valid if version >= BLP1
    pub mipmap_lengths: [u32; 16], // valid if version >= BLP1
}

impl Header {
    pub fn parse(cursor: &mut Cursor<&[u8]>) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let version_raw = cursor.read_u32::<BigEndian>()?;
        let version = Version::try_from(version_raw).map_err(|_| "Invalid BLP version")?;

        let texture_type_raw = cursor.read_u32::<LittleEndian>()?;
        let texture_type = TextureType::try_from(texture_type_raw).map_err(|_| "Invalid BLP version")?;

        let (compression, alpha_bits, alpha_type, has_mips) = if version >= Version::BLP2 {
            (cursor.read_u8()?, cursor.read_u8()? as u32, cursor.read_u8()?, cursor.read_u8()?)
        } else {
            (0, cursor.read_u32::<LittleEndian>()?, 0, 0)
        };

        let width = cursor.read_u32::<LittleEndian>()?;
        let height = cursor.read_u32::<LittleEndian>()?;

        let (extra, has_mipmaps) = if version <= Version::BLP1 {
            (cursor.read_u32::<LittleEndian>()?, cursor.read_u32::<LittleEndian>()?)
        } else {
            (0, has_mips as u32)
        };

        let (mipmap_offsets, mipmap_lengths) = if version >= Version::BLP1 {
            let mut offsets = [0u32; 16];
            let mut lengths = [0u32; 16];
            for i in 0..16 {
                offsets[i] = cursor.read_u32::<LittleEndian>()?;
            }
            for i in 0..16 {
                lengths[i] = cursor.read_u32::<LittleEndian>()?;
            }
            (offsets, lengths)
        } else {
            ([0; 16], [0; 16])
        };

        Ok(Header { version, texture_type, compression, alpha_bits, alpha_type, has_mips, width, height, extra, has_mipmaps, mipmap_offsets, mipmap_lengths })
    }
}
