pub(crate) use crate::header::{HEADER_SIZE, Header};
use crate::mipmap::Mipmap;
use crate::texture_type::TextureType;
use crate::util::center_crop_to_pow2::center_crop_to_pow2;
use byteorder::{LittleEndian, ReadBytesExt};
use image::imageops::{FilterType, resize};
use std::error::Error;
use std::io::{Cursor, Read};

#[derive(Debug)]
pub struct ImageBlp {
    pub header: Header,
    pub mipmaps: Vec<Mipmap>,
    pub holes: usize,
}

impl ImageBlp {
    pub fn from_bytes(buf: &[u8]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        if buf.len() >= 3 && &buf[..3] == b"BLP" {
            // 👉 Тут вызываем настоящий парсер BLP
            return Self::parse_blp(buf);
        }

        // 👉 Иначе пробуем открыть через image crate
        let img = image::load_from_memory(buf)
            .map_err(|e| format!("raster decode failed: {e}"))?
            .to_rgba8();

        let (w, h) = img.dimensions();
        if w == 0 || h == 0 {
            return Err("raster image has zero width or height".into());
        }

        // Центр-кропим под степени двойки
        let cropped = center_crop_to_pow2(&img);

        // Делаем mip-цепочку до 1
        let (mut w, mut h) = cropped.dimensions();
        let mut chain = Vec::with_capacity(16);
        chain.push(cropped.clone());
        while (w > 1 && h > 1) && chain.len() < 16 {
            let nw = (w / 2).max(1);
            let nh = (h / 2).max(1);
            let next = resize(chain.last().unwrap(), nw, nh, FilterType::Triangle);
            chain.push(next);
            w = nw;
            h = nh;
            if w == 1 || h == 1 {
                break; // стоп на 2×1 / 1×N или естественный 1×1 (из квадрата)
            }
        }

        let mut mipmaps: Vec<Mipmap> = chain
            .into_iter()
            .map(|im| {
                let (w, h) = im.dimensions();
                Mipmap { width: w, height: h, image: Some(im) }
            })
            .collect();

        if mipmaps.len() > 16 {
            mipmaps.truncate(16);
        }
        while mipmaps.len() < 16 {
            mipmaps.push(Mipmap::default());
        }

        Ok(ImageBlp { header: Header { width: w, height: h, ..Default::default() }, mipmaps, holes: 0 })
    }

    fn parse_blp(buf: &[u8]) -> Result<Self, Box<dyn Error + Send + Sync>> {
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
            TextureType::JPEG => Self::fill_jpeg(&mut cursor, &header, slices, &mut mipmaps)?,
            TextureType::DIRECT => Self::fill_direct(&mut cursor, &header, slices, &mut mipmaps)?,
        }

        Ok(Self { header, mipmaps, holes })
    }

    fn fill_jpeg(cursor: &mut Cursor<&[u8]>, header: &Header, slices: Vec<Option<&[u8]>>, mipmaps: &mut Vec<Mipmap>) -> Result<(), Box<dyn Error + Send + Sync>> {
        let jpeg_header_size = cursor.read_u32::<LittleEndian>()? as usize;
        let mut jpeg_header_chunk = vec![0u8; jpeg_header_size];
        cursor.read_exact(&mut jpeg_header_chunk)?;

        for (_, slice_opt) in slices.into_iter().enumerate() {
            if let Some(slice) = slice_opt {
                let mipmap = Mipmap::read_jpeg(&jpeg_header_chunk, slice)?;

                let mut level = 0;
                let mut w = header.width;
                let mut h = header.height;
                while level < 16 {
                    if w.max(1) == mipmap.width && h.max(1) == mipmap.height {
                        break;
                    }
                    if w == 0 && h == 0 {
                        break;
                    }
                    w >>= 1;
                    h >>= 1;
                    level += 1;
                }

                if level < 16 {
                    mipmaps[level] = mipmap;
                }
            }
        }

        Ok(())
    }

    fn fill_direct(cursor: &mut Cursor<&[u8]>, header: &Header, slices: Vec<Option<&[u8]>>, mipmaps: &mut Vec<Mipmap>) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut palette = [[0u8; 3]; 256];
        for i in 0..256 {
            let color = cursor.read_u32::<LittleEndian>()?;
            palette[i] = [((color >> 16) & 0xFF) as u8, ((color >> 8) & 0xFF) as u8, (color & 0xFF) as u8];
        }

        for (i, slice_opt) in slices.into_iter().enumerate() {
            if let Some(slice) = slice_opt {
                let mut slice_cursor = Cursor::new(slice);
                let mipmap = Mipmap::read_direct(&mut slice_cursor, mipmaps[i].width, mipmaps[i].height, &palette, header.alpha_bits)?;
                mipmaps[i] = mipmap;
            }
        }

        Ok(())
    }
}
