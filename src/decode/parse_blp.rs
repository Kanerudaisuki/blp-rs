use crate::header::{HEADER_SIZE, Header};
use crate::image_blp::{ImageBlp, MAX_MIPS};
use crate::mipmap::Mipmap;
use crate::texture_type::TextureType;
use byteorder::{LittleEndian, ReadBytesExt};
use std::error::Error;
use std::io::Cursor;

impl ImageBlp {
    pub(crate) fn parse_blp(buf: &[u8]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        // 1) Парсим заголовок
        let mut cursor = Cursor::new(buf);
        let header = Header::parse(&mut cursor)?;

        if cursor.position() != HEADER_SIZE {
            eprintln!("Warning: unexpected cursor position, got {}", cursor.position());
        }

        // 2) Геометрия мипов: считаем по наибольшей стороне до 1×1 (но максимум 16 уровней)
        let w0 = header.width.max(1);
        let h0 = header.height.max(1);
        let max_side = w0.max(h0);
        let mut mip_count = (max_side.ilog2() as usize) + 1;
        mip_count = mip_count.min(MAX_MIPS);

        let mut mipmaps = Vec::with_capacity(mip_count);
        let (mut w, mut h) = (w0, h0);
        for _ in 0..mip_count {
            mipmaps.push(Mipmap { width: w, height: h, image: None });
            if w == 1 && h == 1 {
                break;
            }
            w = (w / 2).max(1);
            h = (h / 2).max(1);
        }
        // На случай если дошли до 1×1 раньше рассчитанного числа уровней
        mip_count = mipmaps.len();

        // 3) Собираем срезы для реально существующих уровней
        let mut slices: Vec<Option<&[u8]>> = vec![None; mip_count];
        let mut ranges: Vec<(usize, usize)> = Vec::with_capacity(mip_count);

        for i in 0..mip_count {
            let offset = header.mipmap_offsets[i] as usize;
            let length = header.mipmap_lengths[i] as usize;

            if length == 0 {
                continue;
            }
            let end = match offset.checked_add(length) {
                Some(e) if e <= buf.len() => e,
                _ => continue, // некорректный диапазон — пропускаем
            };

            slices[i] = Some(&buf[offset..end]);
            ranges.push((offset, end));
        }

        // 4) Подсчёт «дыр» между блоками данных
        ranges.sort_by_key(|r| r.0);

        let mut holes: usize = 0;
        let mut prev_end: usize = match header.texture_type {
            TextureType::JPEG => {
                // Для JPEG после заголовка идут: u32 (размер JPEG-заголовка) + сам JPEG header
                let base = HEADER_SIZE as usize;
                if buf.len() < base + 4 {
                    return Err("Truncated buffer while reading JPEG header size".into());
                }
                // Читаем размер JPEG-заголовка, не сдвигая основной курсор
                let mut c = Cursor::new(&buf[base..]);
                let jpeg_hdr_size = c.read_u32::<LittleEndian>()? as usize;
                base + 4 + jpeg_hdr_size
            }
            TextureType::DIRECT => {
                // После заголовка идёт палитра 256 * 4 байта
                HEADER_SIZE as usize + 256 * 4
            }
        };

        for (start, end) in &ranges {
            if *start < prev_end {
                eprintln!("Warning: overlapping mipmap data detected: [{start}..{end}) overlaps with previous end at {prev_end}");
            } else {
                holes += start.saturating_sub(prev_end);
            }
            if *end > prev_end {
                prev_end = *end;
            }
        }
        // Хвост после последнего блока
        if buf.len() > prev_end {
            holes += buf.len() - prev_end;
        }

        // 5) Декод по типу текстуры
        match header.texture_type {
            TextureType::JPEG => {
                Self::decode_jpeg(&mut cursor, &header, slices, &mut mipmaps)?;
            }
            TextureType::DIRECT => {
                Self::decode_direct(&mut cursor, &header, slices, &mut mipmaps)?;
            }
        }

        Ok(Self { header, mipmaps, holes })
    }
}
