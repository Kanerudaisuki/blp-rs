use crate::encode::blp::options::EncoderCtx;
use crate::encode::blp::report::EncodeReport;
use crate::encode::blp::unit::unit::MipUnit;
use crate::image_blp::MAX_MIPS;
use byteorder::{BigEndian, LittleEndian, WriteBytesExt};

/// Пишем простой BLP-контейнер: для каждого включённого мипа кладём ПОЛНЫЙ JPEG.
/// common_header_len = 0 (ничего общего).
pub fn write_blp_full_jpegs(ctx: &EncoderCtx, mips: &[MipUnit]) -> EncodeReport {
    assert_eq!(mips.len(), MAX_MIPS, "mips must be exactly MAX_MIPS");

    let mut out = Vec::<u8>::new();

    // --- BLP1 заголовок (как в твоих тестах) ---
    out.write_u32::<BigEndian>(0x424C5031)
        .unwrap(); // "BLP1"
    out.write_u32::<LittleEndian>(0)
        .unwrap(); // compression = JPEG
    out.write_u32::<LittleEndian>(8)
        .unwrap(); // alpha_depth (как у Blizzard)
    out.write_u32::<LittleEndian>(ctx.base_width)
        .unwrap();
    out.write_u32::<LittleEndian>(ctx.base_height)
        .unwrap();
    out.write_u32::<LittleEndian>(5)
        .unwrap(); // historical / palette type (из твоего старого кода)
    out.write_u32::<LittleEndian>(0)
        .unwrap(); // hasCommonHeader = 0 (кладём ПОЛНЫЕ JPEG)

    // Таблицы смещений/размеров для 16 мипов
    let offsets_pos = out.len();
    let mut mm_offsets = [0u32; MAX_MIPS];
    let mut mm_sizes = [0u32; MAX_MIPS];
    for _ in 0..MAX_MIPS {
        out.write_u32::<LittleEndian>(0)
            .unwrap();
    }
    for _ in 0..MAX_MIPS {
        out.write_u32::<LittleEndian>(0)
            .unwrap();
    }

    // common header len (0) и никакого header не пишем
    out.write_u32::<LittleEndian>(0)
        .unwrap();

    // --- Сами данные мипов ---
    for i in 0..MAX_MIPS {
        let mu = &mips[i];
        if mu.included && mu.jpeg_full_bytes > 0 {
            mm_offsets[i] = out.len() as u32;
            out.extend_from_slice(&mu.jpeg_full);
            mm_sizes[i] = mu.jpeg_full.len() as u32;
        } else {
            mm_offsets[i] = 0;
            mm_sizes[i] = 0;
        }
    }

    // --- Патчим таблицы offsets/sizes ---
    {
        let mut cur = std::io::Cursor::new(&mut out[offsets_pos..]);
        for off in mm_offsets {
            cur.write_u32::<LittleEndian>(off)
                .unwrap();
        }
        for len in mm_sizes {
            cur.write_u32::<LittleEndian>(len)
                .unwrap();
        }
    }

    // Считаем суммарный полезный объём (для отчёта)
    let total_bytes: usize = mips
        .iter()
        .filter(|m| m.included)
        .map(|m| m.jpeg_full_bytes)
        .sum();

    EncodeReport { bytes: out, base_width: ctx.base_width, base_height: ctx.base_height, first_visible_mip: ctx.first_visible_mip, visible_count: ctx.visible_count, has_alpha: ctx.has_alpha, common_header_len: 0, total_slices_bytes: total_bytes, effective_mip_visible: ctx.eff_visible, mips: mips.to_vec() }
}
