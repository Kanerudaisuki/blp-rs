use crate::core::image::{HEADER_SIZE, ImageBlp, MAX_MIPS};
use crate::core::mipmap::Mipmap;
use crate::core::types::{SourceKind, TextureType, Version};
use crate::error::error::BlpError;
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use std::io::Cursor;

impl ImageBlp {
    pub(crate) fn from_buf_blp(buf: &[u8]) -> Result<Self, BlpError> {
        let mut cursor = Cursor::new(buf);

        let version_raw = cursor.read_u32::<BigEndian>()?;
        let version = Version::try_from(version_raw)?;

        let texture_type_raw = cursor.read_u32::<LittleEndian>()?;
        let texture_type = TextureType::try_from(texture_type_raw)?;

        let (compression, alpha_bits, alpha_type, has_mips) = if version >= Version::BLP2 {
            (
                cursor.read_u8()?,        // compression
                cursor.read_u8()? as u32, // alpha_bits
                cursor.read_u8()?,        // alpha_type
                cursor.read_u8()?,        // has_mips flag
            )
        } else {
            (
                0u8,
                cursor.read_u32::<LittleEndian>()?, // alpha_bits
                0u8,
                0u8,
            )
        };

        let width = cursor.read_u32::<LittleEndian>()?;
        let height = cursor.read_u32::<LittleEndian>()?;

        let (extra, has_mipmaps) = if version <= Version::BLP1 {
            (cursor.read_u32::<LittleEndian>()?, cursor.read_u32::<LittleEndian>()?)
        } else {
            (0u32, has_mips as u32)
        };

        // таблицы смещений/длин
        let mut mipmap_offsets = [0u32; MAX_MIPS];
        let mut mipmap_lengths = [0u32; MAX_MIPS];
        if version >= Version::BLP1 {
            for i in 0..MAX_MIPS {
                mipmap_offsets[i] = cursor.read_u32::<LittleEndian>()?;
            }
            for i in 0..MAX_MIPS {
                mipmap_lengths[i] = cursor.read_u32::<LittleEndian>()?;
            }
        }

        // ширины/высоты для всех MAX_MIPS
        let mut wh = [(1u32, 1u32); MAX_MIPS];
        let (mut w, mut h) = (width.max(1), height.max(1));
        for i in 0..MAX_MIPS {
            wh[i] = (w, h);
            if w > 1 || h > 1 {
                w = (w / 2).max(1);
                h = (h / 2).max(1);
            }
        }

        let mut mipmaps = Vec::with_capacity(MAX_MIPS);
        for i in 0..MAX_MIPS {
            mipmaps.push(Mipmap { width: wh[i].0, height: wh[i].1, image: None, offset: mipmap_offsets[i] as usize, length: mipmap_lengths[i] as usize });
        }

        // header_offset / header_length
        let (header_offset, header_length) = match texture_type {
            TextureType::JPEG => {
                let base = HEADER_SIZE as usize;
                if buf.len() < base + 4 {
                    return Err(BlpError::new("truncated: cannot read JPEG header size"));
                }
                let mut c = Cursor::new(&buf[base..]);
                let hdr_len = c.read_u32::<LittleEndian>()? as usize;
                let hdr_off = base + 4;
                if buf.len() < hdr_off + hdr_len {
                    return Err(BlpError::new("truncated: JPEG header out of bounds"));
                }
                (hdr_off, hdr_len)
            }
            TextureType::DIRECT => {
                // палитра сразу после HEADER_SIZE
                (HEADER_SIZE as usize, 256 * 4)
            }
        };

        // считаем дырки
        let mut ranges = Vec::new();
        for i in 0..MAX_MIPS {
            let off = mipmap_offsets[i] as usize;
            let len = mipmap_lengths[i] as usize;
            if len == 0 {
                continue;
            }
            if let Some(end) = off.checked_add(len) {
                if end <= buf.len() {
                    ranges.push((off, end));
                }
            }
        }
        ranges.sort_by_key(|r| r.0);

        let mut prev_end = header_offset + header_length;
        let mut holes = 0usize;
        for (start, end) in &ranges {
            if *start >= prev_end {
                holes += start - prev_end;
            }
            if *end > prev_end {
                prev_end = *end;
            }
        }
        if buf.len() > prev_end {
            holes += buf.len() - prev_end;
        }

        Ok(Self {
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
            mipmaps,
            holes,
            header_offset,
            header_length,
            source: SourceKind::Blp,
        })
    }
}
