use crate::header::{HEADER_SIZE, Header};
use crate::image_blp::ImageBlp;
use crate::mipmap::Mipmap;
use crate::texture_type::TextureType;
use byteorder::{LittleEndian, ReadBytesExt};
use std::error::Error;
use std::io::Cursor;

impl ImageBlp {
    pub(crate) fn parse_blp(buf: &[u8]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let mut cursor = Cursor::new(buf);
        let header = Header::parse(&mut cursor)?;

        if cursor.position() != HEADER_SIZE {
            eprintln!("Warning: unexpected cursor position, got {}", cursor.position());
        }

        let mut mipmaps = (0..16)
            .map(|i| {
                Mipmap {
                    width: (header.width >> i).max(0), //
                    height: (header.height >> i).max(0),
                    image: None,
                }
            })
            .collect::<Vec<_>>();

        // Собираем срезы и проверяем целостность
        let mut slices = vec![None; 16];
        let mut ranges = Vec::new();

        for i in 0..16 {
            let offset = header.mipmap_offsets[i] as usize;
            let length = header.mipmap_lengths[i] as usize;

            if length == 0 || offset + length > buf.len() {
                continue;
            }

            slices[i] = Some(&buf[offset..offset + length]);
            ranges.push((offset, offset + length));
        }

        ranges.sort_by_key(|r| r.0);
        let mut holes = 0;

        let mut prev_end = match header.texture_type {
            TextureType::JPEG => {
                let saved_pos = cursor.position();

                let size = match cursor.read_u32::<LittleEndian>() {
                    Ok(s) => s as usize,
                    Err(_) => {
                        cursor.set_position(saved_pos); // откат даже если ошибка
                        return Err("Failed to read JPEG header size".into());
                    }
                };

                cursor.set_position(saved_pos); // откат курсора
                saved_pos as usize + 4 + size
            }
            TextureType::DIRECT => (cursor.position() + 256 * 4) as usize,
        };

        for (start, end) in &ranges {
            if *start < prev_end {
                eprintln!("Warning: overlapping mipmap data detected: [{start}..{end}) overlaps with previous end at {prev_end}");
            } else {
                holes += start - prev_end;
            }
            prev_end = prev_end.max(*end);
        }

        holes += buf.len().saturating_sub(prev_end);

        match header.texture_type {
            TextureType::JPEG => Self::decode_jpeg(&mut cursor, &header, slices, &mut mipmaps)?,
            TextureType::DIRECT => Self::decode_direct(&mut cursor, &header, slices, &mut mipmaps)?,
        }

        Ok(Self { header, mipmaps, holes })
    }
}
