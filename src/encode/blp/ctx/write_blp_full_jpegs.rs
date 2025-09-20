// src/encode/blp/ctx/write.rs

use crate::encode::blp::ctx::ctx::EncoderCtx;
use crate::err::error::BlpError;
use crate::image_blp::MAX_MIPS;
use byteorder::{BigEndian, LittleEndian, WriteBytesExt};

impl EncoderCtx {
    /// Пишет простой BLP1-контейнер в `self.bytes` из `self.mips`.
    /// Кладём ПОЛНЫЕ JPEG, `common_header_len = 0`.
    pub fn write_blp_full_jpegs(&mut self) -> Result<(), BlpError> {
        if self.mips.len() != MAX_MIPS {
            return Err(BlpError::new("encode_blp_mips_len"));
        }

        let mut out = Vec::<u8>::new();

        // Header
        out.write_u32::<BigEndian>(0x424C5031)?; // "BLP1"
        out.write_u32::<LittleEndian>(0)
            .map_err(|e| BlpError::new("third_party_error").push_std(e))?; // JPEG
        out.write_u32::<LittleEndian>(8)
            .map_err(|e| BlpError::new("third_party_error").push_std(e))?; // alpha_depth (как у Blizzard)
        out.write_u32::<LittleEndian>(self.base_width)
            .map_err(|e| BlpError::new("third_party_error").push_std(e))?;
        out.write_u32::<LittleEndian>(self.base_height)
            .map_err(|e| BlpError::new("third_party_error").push_std(e))?;
        out.write_u32::<LittleEndian>(5)
            .map_err(|e| BlpError::new("third_party_error").push_std(e))?; // historical / palette type
        out.write_u32::<LittleEndian>(0)
            .map_err(|e| BlpError::new("third_party_error").push_std(e))?; // hasCommonHeader = 0

        // Tables (offsets + sizes)
        let offsets_pos = out.len();
        for _ in 0..(MAX_MIPS * 2) {
            out.write_u32::<LittleEndian>(0)
                .map_err(|e| BlpError::new("third_party_error").push_std(e))?;
        }

        // common header len = 0
        out.write_u32::<LittleEndian>(0)
            .map_err(|e| BlpError::new("third_party_error").push_std(e))?;

        // Payload
        let mut mm_offsets = [0u32; MAX_MIPS];
        let mut mm_sizes = [0u32; MAX_MIPS];

        for i in 0..MAX_MIPS {
            let mu = &self.mips[i];
            if mu.included && mu.jpeg_full_bytes > 0 {
                mm_offsets[i] = out.len() as u32;
                out.extend_from_slice(&mu.jpeg_full);
                mm_sizes[i] = mu.jpeg_full.len() as u32;
            }
        }

        // Patch tables
        {
            let mut cur = std::io::Cursor::new(&mut out[offsets_pos..]);
            for off in mm_offsets {
                cur.write_u32::<LittleEndian>(off)
                    .map_err(|e| BlpError::new("third_party_error").push_std(e))?;
            }
            for len in mm_sizes {
                cur.write_u32::<LittleEndian>(len)
                    .map_err(|e| BlpError::new("third_party_error").push_std(e))?;
            }
        }

        // Totals in ctx
        self.total_slices_bytes = self
            .mips
            .iter()
            .filter(|m| m.included)
            .map(|m| m.jpeg_full_bytes)
            .sum();
        self.common_header_len = 0;
        self.bytes = out;

        Ok(())
    }
}
