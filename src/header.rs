use crate::err::blp_err::BlpErr;
use crate::image_blp::MAX_MIPS;
use crate::texture_type::TextureType;
use crate::version::Version;
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use std::io::Cursor;

pub const HEADER_SIZE: u64 = 156;

#[derive(Debug, Default)]
pub struct Header {
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
}

impl Header {
    pub fn parse(cursor: &mut Cursor<&[u8]>) -> Result<Self, BlpErr> {
        let version_raw = cursor.read_u32::<BigEndian>()?;
        let version = Version::try_from(version_raw).map_err(|e| {
            BlpErr::new("blp.version.invalid")
                .with_arg("got", version_raw) // полезно передать, что пришло
                .with_arg("msg", e.to_string()) // текст первичной ошибки (если есть)
        })?;

        let texture_type_raw = cursor.read_u32::<LittleEndian>()?;
        let texture_type = TextureType::try_from(texture_type_raw).map_err(|e| {
            BlpErr::new("blp.version.invalid")
                .with_arg("got", version_raw) // полезно передать, что пришло
                .with_arg("msg", e.to_string()) // текст первичной ошибки (если есть)
        })?;

        let (compression, alpha_bits, alpha_type, has_mips) = if version >= Version::BLP2 {
            (
                cursor.read_u8()?, //
                cursor.read_u8()? as u32,
                cursor.read_u8()?,
                cursor.read_u8()?,
            )
        } else {
            (
                0, //
                cursor.read_u32::<LittleEndian>()?,
                0,
                0,
            )
        };

        let width = cursor.read_u32::<LittleEndian>()?;
        let height = cursor.read_u32::<LittleEndian>()?;

        let (extra, has_mipmaps) = if version <= Version::BLP1 {
            (cursor.read_u32::<LittleEndian>()?, cursor.read_u32::<LittleEndian>()?)
        } else {
            (0, has_mips as u32)
        };

        let (mipmap_offsets, mipmap_lengths) = if version >= Version::BLP1 {
            let mut offsets = [0u32; MAX_MIPS];
            let mut lengths = [0u32; MAX_MIPS];
            for i in 0..MAX_MIPS {
                offsets[i] = cursor.read_u32::<LittleEndian>()?;
            }
            for i in 0..MAX_MIPS {
                lengths[i] = cursor.read_u32::<LittleEndian>()?;
            }
            (offsets, lengths)
        } else {
            ([0; MAX_MIPS], [0; MAX_MIPS])
        };

        Ok(Header {
            version, //
            texture_type,
            compression,
            alpha_bits,
            alpha_type,
            has_mips,
            width,
            height,
            extra,
            has_mipmaps,
            mipmap_offsets,
            mipmap_lengths,
        })
    }
}
