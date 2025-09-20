use crate::encode::blp::ctx::ctx::EncoderCtx;
use crate::err::error::BlpError;
use crate::image_blp::MAX_MIPS;
use byteorder::{BigEndian, LittleEndian, WriteBytesExt};

impl EncoderCtx {
    /// Пишет простой BLP1 контейнер в `self.bytes` из `self.mips`.
    /// Кладём ПОЛНЫЕ JPEG, общий заголовок пока не выносим.
    pub fn write_blp_full_jpegs(&mut self) -> Result<(), BlpError> {
        if self.mips.len() != MAX_MIPS {
            return Err(BlpError::new("encode_blp_mips_len"));
        }

        let mut out = Vec::<u8>::new();

        // --- BLP1 header ---
        out.write_u32::<BigEndian>(0x424C5031)?; // "BLP1"
        out.write_u32::<LittleEndian>(0)?; // compression = JPEG

        // alpha_depth: 0 если нет альфы, 8 если есть (альфа в K-канале)
        let alpha_depth: u32 = if self.has_alpha { 8 } else { 0 };
        out.write_u32::<LittleEndian>(alpha_depth)?;

        out.write_u32::<LittleEndian>(self.base_width)?; // width
        out.write_u32::<LittleEndian>(self.base_height)?; // height
        out.write_u32::<LittleEndian>(5)?; // historical / palette type

        // hasCommonHeader: 1 если есть общий заголовок, иначе 0
        let has_common_header: u32 = if self.common_header_len > 0 { 1 } else { 0 };
        out.write_u32::<LittleEndian>(has_common_header)?;

        // Резерв под таблицы offsets и sizes (по MAX_MIPS u32 каждая)
        let offsets_pos = out.len();
        for _ in 0..(MAX_MIPS * 2) {
            out.write_u32::<LittleEndian>(0)?;
        }

        // Длина общего заголовка (кладём значение из контекста)
        out.write_u32::<LittleEndian>(self.common_header_len as u32)?;
        // Примечание: сами байты общего заголовка не пишем (на текущей фазе len=0).

        // --- Payload: полные JPEG по включённым мипам ---
        let mut mm_offsets = [0u32; MAX_MIPS];
        let mut mm_sizes = [0u32; MAX_MIPS];

        for (i, mu) in self
            .mips
            .iter()
            .enumerate()
            .take(MAX_MIPS)
        {
            if mu.included && mu.jpeg_full_bytes > 0 {
                mm_offsets[i] = out.len() as u32;
                out.extend_from_slice(&mu.jpeg_full);
                mm_sizes[i] = mu.jpeg_full_bytes as u32;
            }
        }

        // --- Патчим таблицы offsets/sizes ---
        {
            let mut cur = std::io::Cursor::new(&mut out[offsets_pos..]);
            for off in mm_offsets {
                cur.write_u32::<LittleEndian>(off)?;
            }
            for len in mm_sizes {
                cur.write_u32::<LittleEndian>(len)?;
            }
        }

        // Итоги в контекст
        self.total_slices_bytes = self
            .mips
            .iter()
            .filter(|m| m.included)
            .map(|m| m.jpeg_full_bytes)
            .sum();
        self.bytes = out;

        Ok(())
    }
}
