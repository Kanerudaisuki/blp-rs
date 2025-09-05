use byteorder::{BigEndian, LittleEndian, WriteBytesExt};
use image::{RgbaImage, imageops::FilterType};
use mozjpeg as mj;
use std::error::Error;
use std::io::Write;

use crate::image_blp::ImageBlp;
use crate::mipmap::Mipmap;

impl ImageBlp {
    pub fn encode_blp(&self, quality: u8) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        let base: &RgbaImage = self
            .mipmaps
            .get(0)
            .and_then(|m| m.image.as_ref())
            .ok_or("encode_blp: mip0 RGBA image is missing")?;
        let (w0, h0) = (base.width(), base.height());
        if w0 == 0 || h0 == 0 {
            return Err("encode_blp: invalid base size".into());
        }

        let gen_mips = build_mips_from_first(base, 16)?;

        // кодируем каждый мип в ПОЛНЫЙ 4-канальный JPEG (каналы = ARGB)
        let mut jpegs: Vec<Vec<u8>> = Vec::with_capacity(gen_mips.len());
        for mip in &gen_mips {
            let img = mip
                .image
                .as_ref()
                .ok_or("encode_blp: mip image missing")?;
            jpegs.push(encode_argb_jpeg_full(img, mip.width, mip.height, quality)?);
        }

        // общий заголовок = общий префикс всех JPEG (без лимитов)
        let hdr_len = common_prefix_len(&jpegs);
        if hdr_len < 2 || jpegs[0][0] != 0xFF || jpegs[0][1] != 0xD8 {
            return Err("encode_blp: not a JPEG (no SOI)".into());
        }
        let common_header = jpegs[0][..hdr_len].to_vec();

        // контейнер BLP1/JPEG
        let mut out = Vec::new();
        out.write_u32::<BigEndian>(0x424C5031)?; // "BLP1"
        out.write_u32::<LittleEndian>(0)?; // content = JPEG
        out.write_u32::<LittleEndian>(8)?; // как в твоём header
        out.write_u32::<LittleEndian>(w0)?;
        out.write_u32::<LittleEndian>(h0)?;
        out.write_u32::<LittleEndian>(5)?; // historical
        out.write_u32::<LittleEndian>(if jpegs.len() > 1 { 1 } else { 0 })?;

        // таблицы offsets/sizes
        let offsets_pos = out.len();
        let mut mm_offsets = [0u32; 16];
        let mut mm_sizes = [0u32; 16];
        for _ in 0..16 {
            out.write_u32::<LittleEndian>(0)?;
        }
        for _ in 0..16 {
            out.write_u32::<LittleEndian>(0)?;
        }

        // общий JPEG header
        out.write_u32::<LittleEndian>(common_header.len() as u32)?;
        out.write_all(&common_header)?;

        // хвосты (slices)
        for (i, jpeg) in jpegs.iter().enumerate() {
            let slice = &jpeg[hdr_len..];
            mm_offsets[i] = out.len() as u32;
            out.write_all(slice)?;
            mm_sizes[i] = slice.len() as u32;
        }

        // пропатчить таблицы
        {
            let mut cur = std::io::Cursor::new(&mut out[offsets_pos..]);
            for off in mm_offsets {
                cur.write_u32::<LittleEndian>(off)?;
            }
            for len in mm_sizes {
                cur.write_u32::<LittleEndian>(len)?;
            }
        }

        Ok(out)
    }
}

// ----------------- helpers -----------------
fn build_mips_from_first(base: &RgbaImage, max_levels: usize) -> Result<Vec<Mipmap>, Box<dyn Error + Send + Sync>> {
    if max_levels == 0 {
        return Err("build_mips_from_first: max_levels must be > 0".into());
    }

    let (mut w, mut h) = base.dimensions();
    if w == 0 || h == 0 {
        return Err("build_mips_from_first: base has zero dimension".into());
    }

    let mut chain: Vec<Mipmap> = Vec::with_capacity(max_levels);
    chain.push(Mipmap { width: w, height: h, image: Some(base.clone()) });

    // ↓ ключевая правка: условие до 1×1 (НЕ &&, а ||)
    while (w > 1 || h > 1) && chain.len() < max_levels {
        let nw = (w / 2).max(1);
        let nh = (h / 2).max(1);
        let prev = chain
            .last()
            .unwrap()
            .image
            .as_ref()
            .unwrap();
        let next = image::imageops::resize(prev, nw, nh, FilterType::Triangle);
        chain.push(Mipmap { width: nw, height: nh, image: Some(next) });
        w = nw;
        h = nh;
    }

    Ok(chain)
}

/// Пишем в JPEG 4 компонента **в порядке BGRA** без каких-либо преобразований содержимого.
/// (Просто сырой 4-канальный поток; сэмплинг 1×1, без optimize, чтобы префикс совпадал.)
fn encode_argb_jpeg_full(img: &RgbaImage, w: u32, h: u32, quality: u8) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    // упаковываем строго A,R,G,B
    let mut argb = Vec::with_capacity((w as usize * h as usize) * 4);
    for p in img.pixels() {
        argb.extend_from_slice(&[p[2], p[1], p[0], p[3]]); // BGRA
    }

    // важное: просим 4-компонентный JPEG без цветовых трансформов.
    // В mozjpeg это достигается через JCS_CMYK как «контейнер» 4-комп. и 4:4:4;
    // данные кладём как есть (ARGB уже в тех 4 байтах).
    let mut comp = mj::Compress::new(mj::ColorSpace::JCS_CMYK);
    comp.set_size(w as usize, h as usize);
    comp.set_quality(quality as f32);
    comp.set_optimize_coding(false); // одинаковые таблицы → общий префикс длиннее
    for c in comp.components_mut().iter_mut() {
        c.h_samp_factor = 1;
        c.v_samp_factor = 1; // 4:4:4
    }

    let mut started = comp.start_compress(Vec::new())?;
    started.write_scanlines(&argb)?;
    let jpeg_buf: Vec<u8> = started.finish()?;
    Ok(jpeg_buf)
}

/// общий префикс байт по всем JPEG (как в твоём Haskell scanHeader), без лимита
fn common_prefix_len(jpegs: &[Vec<u8>]) -> usize {
    if jpegs.is_empty() {
        return 0;
    }
    if jpegs.len() == 1 {
        return jpegs[0].len();
    }
    let min_len = jpegs
        .iter()
        .map(|v| v.len())
        .min()
        .unwrap();
    let mut i = 0usize;
    while i < min_len {
        let b0 = jpegs[0][i];
        if jpegs.iter().skip(1).any(|j| j[i] != b0) {
            break;
        }
        i += 1;
    }
    i
}
