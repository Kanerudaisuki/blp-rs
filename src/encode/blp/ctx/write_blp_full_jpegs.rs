// src/encode/blp/ctx/write.rs  (или где у тебя метод контекста)
use crate::encode::blp::ctx::ctx::EncoderCtx;
use byteorder::{BigEndian, LittleEndian, WriteBytesExt};
// Common header holds full JPEG header up to and including SOS (from mip0).
// Each mip slice holds only the entropy scan; legacy path appends EOI when we
// cannot share a plan across mips.
use crate::err::error::BlpError;
use crate::image_blp::MAX_MIPS;

impl EncoderCtx {
    /// Пишет BLP1: общий JPEG-хедер (`self.common_header`) + массивы чистых scan-слайсов.
    pub fn write_blp_common_header_and_slices(&mut self) -> Result<(), BlpError> {
        if self.mips.len() != MAX_MIPS {
            return Err(BlpError::new("encode_blp_mips_len"));
        }
        // allow empty common header (header_len = 0) when storing full JPEG per mip

        let mut out = Vec::<u8>::new();

        // --- BLP1 header (156 bytes total) ---
        // Layout must match src/header.rs::Header::parse for BLP1
        out.write_u32::<BigEndian>(0x424C5031)?; // version = "BLP1"
        out.write_u32::<LittleEndian>(0)?; // texture_type = JPEG (0)
        out.write_u32::<LittleEndian>(self.alpha_depth as u32)?; // alpha_bits (0 or 8)
        out.write_u32::<LittleEndian>(self.base_width)?; // width
        out.write_u32::<LittleEndian>(self.base_height)?; // height
        out.write_u32::<LittleEndian>(0)?; // extra (BLP1 legacy)
        out.write_u32::<LittleEndian>(0)?; // has_mipmaps (BLP1 legacy)

        // mip offsets + mip sizes (reserved, patch later)
        let offsets_pos = out.len();
        for _ in 0..(MAX_MIPS * 2) {
            out.write_u32::<LittleEndian>(0)?;
        }

        // common header len + bytes (must be non-zero for header+scan scheme)
        out.write_u32::<LittleEndian>(self.common_header_len as u32)?;
        out.extend_from_slice(&self.common_header);

        let plan_opt = self.jpeg_plan.as_ref();

        // --- payload: только SCAN каждого включённого мипа ---
        let mut mm_offsets = [0u32; MAX_MIPS];
        let mut mm_sizes = [0u32; MAX_MIPS];

        // Для каждого мипа кладём только entropy-скан (+ EOI)

        for (i, mu) in self
            .mips
            .iter()
            .enumerate()
            .take(MAX_MIPS)
        {
            if !mu.included {
                continue;
            }
            let Some(slc) = mu.jpeg_slices else {
                return Err(BlpError::new("encode_blp_missing_slices").with_arg("mip", i as u32));
            };
            let start = slc.head_len;
            let end = slc.head_len + slc.scan_len;
            if end > mu.jpeg_full_bytes {
                return Err(BlpError::new("encode_blp_slice_oob")
                    .with_arg("mip", i as u32)
                    .with_arg("head_len", slc.head_len as u32)
                    .with_arg("scan_len", slc.scan_len as u32)
                    .with_arg("jpeg_full_bytes", mu.jpeg_full_bytes as u32));
            }
            mm_offsets[i] = out.len() as u32;
            if plan_opt.is_some() {
                out.extend_from_slice(&mu.jpeg_full[start..end]);
                mm_sizes[i] = slc.scan_len as u32;
            } else {
                out.extend_from_slice(&mu.jpeg_full[start..end]);
                out.extend_from_slice(&[0xFF, 0xD9]);
                mm_sizes[i] = (slc.scan_len as u32) + 2;
            }
        }

        // патчим таблицы
        {
            let mut cur = std::io::Cursor::new(&mut out[offsets_pos..]);
            for off in mm_offsets {
                cur.write_u32::<LittleEndian>(off)?;
            }
            for len in mm_sizes {
                cur.write_u32::<LittleEndian>(len)?;
            }
        }

        // итоги
        self.total_slices_bytes = if plan_opt.is_some() {
            self.mips
                .iter()
                .filter_map(|m| {
                    m.jpeg_slices
                        .as_ref()
                        .map(|s| if m.included { s.scan_len } else { 0 })
                })
                .map(|v| v)
                .sum()
        } else {
            self.mips
                .iter()
                .filter(|m| m.included)
                .map(|m| m.jpeg_full_bytes)
                .sum()
        };

        self.bytes = out;
        Ok(())
    }
}
