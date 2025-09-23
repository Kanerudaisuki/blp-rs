use crate::core::encode::utils::pack_bgr_from_rgba::pack_bgr_from_rgba;
use crate::core::encode::utils::pack_cmyk_from_rgba::pack_cmyk_from_rgba;
use crate::core::encode::utils::tj3_compress_cmyk::tj3_compress_cmyk;
use crate::core::encode::utils::tj3_compress_ycbcr_from_rgb::tj3_compress_ycbcr_from_rgb;
use crate::core::encode::utils::tj3_err::tj3_err;
use crate::core::image::{ImageBlp, MAX_MIPS};
use crate::error::error::BlpError;
use image::{DynamicImage, RgbaImage, imageops::FilterType};
use turbojpeg::{libc, raw};

// ============ Публичный вход ============
pub struct EncoderCtx {
    pub bytes: Vec<u8>,
}

impl ImageBlp {
    pub fn encode_blp(&self, quality: u8, mip_visible: &[bool]) -> Result<EncoderCtx, BlpError> {
        let base: RgbaImage = self
            .mipmaps
            .get(0)
            .and_then(|m| m.image.clone())
            .ok_or_else(|| BlpError::new("no_base_image"))?;

        let has_alpha = base.pixels().any(|p| p.0[3] != 255);

        if has_alpha {
            encode_blp_impl(
                &base,
                quality,
                mip_visible,
                /*has_alpha=*/ true,
                pack_cmyk_from_rgba, // RGBA -> CMYK (K = 255-A)
                tj3_compress_cmyk,   // CMYK, 4:4:4
            )
        } else {
            encode_blp_impl(
                &base,
                quality,
                mip_visible,
                /*has_alpha=*/ false,
                //pack_bgr_from_rgba, // RGBA -> BGR (как договаривались)
                //tj3_compress_rgb_from_bgr, // RGB colorspace, вход TJPF_BGR
                pack_bgr_from_rgba,
                tj3_compress_ycbcr_from_rgb,
            )
        }
    }
}

// ============ Общая реализация (без дублирования) ============

type PackFn = fn(&RgbaImage) -> (Vec<u8>, usize /*pitch bytes*/);
type CompressFn = fn(&[u8], usize, usize, usize, i32) -> Result<Vec<u8>, BlpError>;

fn encode_blp_impl(base_rgba: &RgbaImage, quality: u8, mip_visible: &[bool], has_alpha: bool, pack: PackFn, compress: CompressFn) -> Result<EncoderCtx, BlpError> {
    // 1) размеры/маска и первый видимый
    let (dims, first_vis) = build_dims_and_first_vis(base_rgba, mip_visible)?;
    // 2) гоним мип-цепочку и кодируем каждый видимый уровень
    let (enc_by_slot, slot_count) = encode_visible_levels(base_rgba, &dims, first_vis, quality, pack, compress)?;
    // 3) финальная сборка BLP
    let bytes = finalize_blp_write(enc_by_slot, slot_count, &dims, first_vis, has_alpha)?;
    Ok(EncoderCtx { bytes })
}

fn encode_visible_levels(base_rgba: &RgbaImage, dims: &[(u32, u32, bool)], first_vis: usize, quality: u8, pack: PackFn, compress: CompressFn) -> Result<(Vec<Option<Enc>>, usize), BlpError> {
    let mut rgba_cur = base_rgba.clone();
    let mut enc_by_slot: Vec<Option<Enc>> = vec![None; MAX_MIPS];
    let mut slot_count = 0usize;

    for (lvl, &(dw, dh, vis)) in dims.iter().enumerate() {
        if lvl > 0 {
            let dyn1 = DynamicImage::ImageRgba8(rgba_cur).resize_exact(dw, dh, FilterType::Lanczos3);
            rgba_cur = dyn1.to_rgba8();
        }
        if lvl < first_vis {
            continue;
        }

        let slot = lvl - first_vis;
        if slot >= MAX_MIPS {
            break;
        }
        slot_count = slot_count.max(slot + 1);

        if !vis {
            enc_by_slot[slot] = None;
            continue;
        }

        let (packed, pitch) = pack(&rgba_cur);
        let (wz, hz) = (rgba_cur.width() as usize, rgba_cur.height() as usize);
        let jpeg_raw = compress(&packed, wz, hz, pitch, quality as i32)?;

        let (head_len, _scan_len) = split_header_and_scan(&jpeg_raw)?;
        // минимальная уборка мусора: выкидываем APPn/COM и двигаем SOF перед SOS
        let header_clean = rebuild_header_min(&jpeg_raw[..head_len])?;
        let mut rebuilt = Vec::with_capacity(jpeg_raw.len());
        rebuilt.extend_from_slice(&header_clean);
        rebuilt.extend_from_slice(&jpeg_raw[head_len..]); // scan + EOI

        let (h2, _s2) = split_header_and_scan(&rebuilt)?;
        enc_by_slot[slot] = Some(Enc { data: rebuilt, sl: Slice { head_len: h2 } });
    }

    if enc_by_slot
        .get(0)
        .and_then(|x| x.as_ref())
        .is_none()
    {
        return Err(BlpError::new("first_visible_slot_missing"));
    }
    Ok((enc_by_slot, slot_count))
}

// ============ TurboJPEG обёртки ============

pub(crate) fn tj3_base_config(handle: raw::tjhandle, q: i32) -> Result<(), BlpError> {
    unsafe {
        if raw::tj3Set(handle, raw::TJPARAM_TJPARAM_QUALITY as libc::c_int, q) != 0 {
            return Err(tj3_err(handle, "tj3.quality"));
        }
        if raw::tj3Set(handle, raw::TJPARAM_TJPARAM_SUBSAMP as libc::c_int, raw::TJSAMP_TJSAMP_444 as libc::c_int) != 0 {
            return Err(tj3_err(handle, "tj3.subsamp"));
        }
        if raw::tj3Set(handle, raw::TJPARAM_TJPARAM_OPTIMIZE as libc::c_int, 0) != 0 {
            return Err(tj3_err(handle, "tj3.optimize"));
        }
        Ok(())
    }
}

// ============ Мелкие общие утилиты ============

#[derive(Clone)]
struct Slice {
    head_len: usize,
}
#[derive(Clone)]
struct Enc {
    data: Vec<u8>,
    sl: Slice,
}

fn build_dims_and_first_vis(rgba0: &RgbaImage, mip_visible: &[bool]) -> Result<(Vec<(u32, u32, bool)>, usize), BlpError> {
    let (mut w, mut h) = (rgba0.width(), rgba0.height());
    let mut dims: Vec<(u32, u32, bool)> = Vec::with_capacity(MAX_MIPS);
    for i in 0..MAX_MIPS {
        let vis = mip_visible
            .get(i)
            .copied()
            .unwrap_or(true);
        dims.push((w, h, vis));
        if w == 1 && h == 1 {
            break;
        }
        w = (w / 2).max(1);
        h = (h / 2).max(1);
    }
    let first_vis = dims
        .iter()
        .position(|&(_, _, v)| v)
        .ok_or_else(|| BlpError::new("no_visible_mips_after_mask"))?;
    Ok((dims, first_vis))
}

fn be_u16(b: &[u8]) -> Result<usize, BlpError> {
    if b.len() < 2 {
        return Err(BlpError::new("jpeg.len"));
    }
    Ok(((b[0] as usize) << 8) | b[1] as usize)
}

fn split_header_and_scan(jpeg: &[u8]) -> Result<(usize, usize), BlpError> {
    if jpeg.len() < 4 || jpeg[0] != 0xFF || jpeg[1] != 0xD8 {
        return Err(BlpError::new("jpeg.bad_soi"));
    }
    let mut i = 2usize;
    loop {
        while i < jpeg.len() && jpeg[i] == 0xFF {
            i += 1;
        }
        if i >= jpeg.len() {
            return Err(BlpError::new("jpeg.truncated"));
        }
        let m = jpeg[i];
        i += 1;
        match m {
            0xD9 => return Err(BlpError::new("jpeg.eoi_before_sos")),
            0xD0..=0xD7 | 0x01 => {} // no length
            0xDA => {
                if i + 2 > jpeg.len() {
                    return Err(BlpError::new("jpeg.sos_len"));
                }
                let seg_len = be_u16(&jpeg[i..i + 2])?;
                let seg_end = i + seg_len;
                if seg_end > jpeg.len() {
                    return Err(BlpError::new("jpeg.sos_trunc"));
                }
                let head_len = seg_end;
                let mut j = head_len;
                while j + 1 < jpeg.len() {
                    if jpeg[j] == 0xFF {
                        let n = jpeg[j + 1];
                        if n == 0x00 || (0xD0..=0xD7).contains(&n) {
                            j += 2;
                            continue;
                        }
                        if n == 0xD9 {
                            return Ok((head_len, j - head_len));
                        }
                    }
                    j += 1;
                }
                return Err(BlpError::new("jpeg.eoi_not_found"));
            }
            _ => {
                if i + 2 > jpeg.len() {
                    return Err(BlpError::new("jpeg.seg_len"));
                }
                let seg_len = be_u16(&jpeg[i..i + 2])?;
                let seg_end = i + seg_len;
                if seg_end > jpeg.len() {
                    return Err(BlpError::new("jpeg.seg_trunc"));
                }
                i = seg_end;
            }
        }
    }
}

fn common_prefix(heads: &[&[u8]]) -> Vec<u8> {
    if heads.is_empty() {
        return Vec::new();
    }
    let min_len = heads
        .iter()
        .map(|h| h.len())
        .min()
        .unwrap();
    let mut out = Vec::with_capacity(min_len);
    for i in 0..min_len {
        let b = heads[0][i];
        if heads.iter().all(|h| h[i] == b) {
            out.push(b);
        } else {
            break;
        }
    }
    out
}

fn finalize_blp_write(enc_by_slot: Vec<Option<Enc>>, slot_count: usize, dims: &[(u32, u32, bool)], first_vis: usize, has_alpha: bool) -> Result<Vec<u8>, BlpError> {
    // общий header как общий префикс
    let mut heads: Vec<&[u8]> = Vec::new();
    for i in 0..slot_count {
        if let Some(e) = &enc_by_slot[i] {
            heads.push(&e.data[..e.sl.head_len]);
        }
    }
    if heads.is_empty() {
        return Err(BlpError::new("no_encoded_heads"));
    }
    let mut common_header = common_prefix(&heads);
    if common_header.len() < 2 || common_header[0] != 0xFF || common_header[1] != 0xD8 {
        return Err(BlpError::new("bad_common_header"));
    }
    for h in &heads {
        while !h.starts_with(&common_header) && !common_header.is_empty() {
            common_header.pop();
        }
        if !h.starts_with(&common_header) {
            return Err(BlpError::new("head_prefix_mismatch"));
        }
    }

    // вырезаем общий head
    #[derive(Clone)]
    struct Block<'a> {
        len: u32,
        bytes: &'a [u8],
    }
    let mut blocks: Vec<Option<Block>> = vec![None; enc_by_slot.len()];
    for i in 0..slot_count {
        if let Some(e) = &enc_by_slot[i] {
            let head = &e.data[..e.sl.head_len];
            let trimmed = &head[common_header.len()..];
            let payload = &e.data[head.len() - trimmed.len()..];
            blocks[i] = Some(Block { len: payload.len() as u32, bytes: payload });
        }
    }

    let base_w = dims[first_vis].0;
    let base_h = dims[first_vis].1;
    let flags: u32 = if has_alpha { 8 } else { 0 };
    let compression: u32 = 0; // JPEG
    let extra_field: u32 = 5;
    let has_mipmaps: u32 = 1;

    let mut offsets = [0u32; MAX_MIPS];
    let mut sizes = [0u32; MAX_MIPS];

    let blp_header_size = 4 + 4 + 4 + 4 + 4 + 4 + 4 + (MAX_MIPS as u32) * 4 + (MAX_MIPS as u32) * 4;
    let jpeg_header_block_size = 4 + (common_header.len() as u32);

    let mut cur = blp_header_size + jpeg_header_block_size;
    for i in 0..MAX_MIPS.min(blocks.len()) {
        if let Some(b) = &blocks[i] {
            offsets[i] = cur;
            sizes[i] = b.len;
            cur = cur
                .checked_add(b.len)
                .ok_or_else(|| BlpError::new("offset_overflow"))?;
        }
    }

    let mut out = Vec::with_capacity(cur as usize);
    out.extend_from_slice(b"BLP1");
    out.extend_from_slice(&compression.to_le_bytes());
    out.extend_from_slice(&flags.to_le_bytes());
    out.extend_from_slice(&base_w.to_le_bytes());
    out.extend_from_slice(&base_h.to_le_bytes());
    out.extend_from_slice(&extra_field.to_le_bytes());
    out.extend_from_slice(&has_mipmaps.to_le_bytes());
    for &off in &offsets {
        out.extend_from_slice(&off.to_le_bytes());
    }
    for &sz in &sizes {
        out.extend_from_slice(&sz.to_le_bytes());
    }
    out.extend_from_slice(&(common_header.len() as u32).to_le_bytes());
    out.extend_from_slice(&common_header);
    for i in 0..MAX_MIPS.min(blocks.len()) {
        if let Some(b) = &blocks[i] {
            out.extend_from_slice(b.bytes);
        }
    }
    Ok(out)
}

// ============ Санитайзер header (минимальный) ============

fn is_app(m: u8) -> bool {
    (0xE0..=0xEF).contains(&m)
}
fn is_com(m: u8) -> bool {
    m == 0xFE
}
fn is_sof(m: u8) -> bool {
    matches!(m, 0xC0..=0xCF) && m != 0xC4 && m != 0xC8
}

/// На вход — SOI..SOS(inclusive). Возврат: SOI + [без APPn/COM] + SOF (первый) + SOS.
fn rebuild_header_min(header: &[u8]) -> Result<Vec<u8>, BlpError> {
    if header.len() < 4 || header[0] != 0xFF || header[1] != 0xD8 {
        return Err(BlpError::new("jpeg.bad_soi"));
    }
    let mut pos = 2usize;
    let mut others: Vec<(usize, usize)> = Vec::new();
    let mut sof_seg: Option<(usize, usize)> = None;
    let mut sos_seg: Option<(usize, usize)> = None;

    while pos < header.len() {
        while pos < header.len() && header[pos] == 0xFF {
            pos += 1;
        }
        if pos >= header.len() {
            break;
        }
        let id = header[pos];
        let start = pos - 1;
        pos += 1;

        if id == 0x01 || (0xD0..=0xD7).contains(&id) {
            others.push((start, pos));
            continue;
        }
        if pos + 2 > header.len() {
            return Err(BlpError::new("jpeg.seg_len"));
        }
        let seg_len = be_u16(&header[pos..pos + 2])?;
        let end = pos + seg_len;
        if end > header.len() {
            return Err(BlpError::new("jpeg.seg_trunc"));
        }

        match id {
            0xDA => {
                sos_seg = Some((start, end));
                break;
            }
            _ if is_app(id) || is_com(id) => {}
            _ if is_sof(id) => {
                if sof_seg.is_none() {
                    sof_seg = Some((start, end));
                }
            }
            _ => {
                others.push((start, end));
            }
        }
        pos = end;
    }

    let (sos_s, sos_e) = sos_seg.ok_or_else(|| BlpError::new("jpeg.sos_missing"))?;
    let (sof_s, sof_e) = sof_seg.ok_or_else(|| BlpError::new("jpeg.sof_missing"))?;

    let mut out = Vec::with_capacity(header.len());
    out.extend_from_slice(&header[..2]);
    for (s, e) in others {
        out.extend_from_slice(&header[s..e]);
    }
    out.extend_from_slice(&header[sof_s..sof_e]);
    out.extend_from_slice(&header[sos_s..sos_e]);
    Ok(out)
}
