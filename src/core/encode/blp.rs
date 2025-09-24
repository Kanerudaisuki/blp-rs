use crate::core::image::{ImageBlp, MAX_MIPS};
use crate::error::error::BlpError;
use image::{DynamicImage, RgbaImage, imageops::FilterType};
use std::ffi::CStr;
use std::ptr;
use turbojpeg::{libc, raw};

pub struct EncoderCtx {
    pub bytes: Vec<u8>,
}

#[derive(Clone)]
struct Enc {
    data: Vec<u8>,
    sl: Slice,
}

#[derive(Clone)]
struct Slice {
    head_len: usize,
}

impl ImageBlp {
    pub fn encode_blp(&self, quality: u8, mip_visible: &[bool]) -> Result<EncoderCtx, BlpError> {
        // 0) базовый мип
        let base: RgbaImage = self
            .mipmaps
            .get(0)
            .and_then(|m| m.image.clone())
            .ok_or_else(|| BlpError::new("no_base_image"))?;

        // 1) есть ли альфа?
        let has_alpha = base.pixels().any(|p| p.0[3] != 255);

        // 3) локально считаем размеры всех мипов и first_vis (без отдельных функций)
        let (mut w, mut h) = (base.width(), base.height());
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

        // 4) кодируем видимые уровни (всё здесь)
        let mut rgba_cur = base.clone();
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

            let (wz, hz) = (rgba_cur.width() as usize, rgba_cur.height() as usize);
            let src = &rgba_cur.as_raw();
            let (packed, pitch) = if has_alpha {
                let mut out = vec![0u8; wz * hz * 4];
                for (dst, px) in out
                    .chunks_exact_mut(4)
                    .zip(src.chunks_exact(4))
                {
                    dst[0] = 255u8.saturating_sub(px[0]); // C = 255-R
                    dst[1] = 255u8.saturating_sub(px[1]); // M = 255-G
                    dst[2] = 255u8.saturating_sub(px[2]); // Y = 255-B
                    dst[3] = 255u8.saturating_sub(px[3]); // K = 255-A
                }
                (out, wz * 4)
            } else {
                let mut out = vec![0u8; wz * hz * 3];
                for (dst, px) in out
                    .chunks_exact_mut(3)
                    .zip(src.chunks_exact(4))
                {
                    dst[0] = px[0]; // B
                    dst[1] = px[1]; // G
                    dst[2] = px[2]; // R
                }
                (out, wz * 3)
            };

            let handle = unsafe { raw::tj3Init(raw::TJINIT_TJINIT_COMPRESS as libc::c_int) };
            if handle.is_null() {
                return Err(BlpError::new("tj3.init"));
            }

            let jpeg_raw = unsafe {
                struct Guard(raw::tjhandle);
                impl Drop for Guard {
                    fn drop(&mut self) {
                        if !self.0.is_null() {
                            unsafe { raw::tj3Destroy(self.0) };
                        }
                    }
                }
                let _g = Guard(handle);

                if raw::tj3Set(handle, raw::TJPARAM_TJPARAM_QUALITY as libc::c_int, quality as libc::c_int) != 0 {
                    return Err(tj3_err(handle, "tj3.quality"));
                }
                if raw::tj3Set(handle, raw::TJPARAM_TJPARAM_SUBSAMP as libc::c_int, raw::TJSAMP_TJSAMP_444 as libc::c_int) != 0 {
                    return Err(tj3_err(handle, "tj3.subsamp"));
                }
                if raw::tj3Set(handle, raw::TJPARAM_TJPARAM_OPTIMIZE as libc::c_int, 0) != 0 {
                    return Err(tj3_err(handle, "tj3.optimize"));
                }

                let mut out_ptr: *mut libc::c_uchar = ptr::null_mut();
                let mut out_size: raw::size_t = 0;

                if raw::tj3Set(
                    handle, //
                    raw::TJPARAM_TJPARAM_COLORSPACE as libc::c_int,
                    if has_alpha {
                        raw::TJCS_TJCS_CMYK as libc::c_int //
                    } else {
                        raw::TJCS_TJCS_RGB as libc::c_int
                    },
                ) != 0
                {
                    return Err(tj3_err(handle, "tj3.colorspace"));
                }

                let r = raw::tj3Compress8(
                    handle, //
                    packed.as_ptr(),
                    wz as libc::c_int,
                    pitch as libc::c_int,
                    hz as libc::c_int,
                    if has_alpha {
                        raw::TJPF_TJPF_CMYK as libc::c_int // вход: CMYK
                    } else {
                        raw::TJPF_TJPF_BGR as libc::c_int // вход: BGR
                    },
                    &mut out_ptr,
                    &mut out_size,
                );
                if r != 0 {
                    return Err(tj3_err(handle, "tj3.compress"));
                }
                let slice = std::slice::from_raw_parts(out_ptr, out_size as usize);
                let vec = slice.to_vec();
                raw::tj3Free(out_ptr as *mut libc::c_void);
                vec
            };

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

        // 5) финальная сборка контейнера BLP
        let bytes = finalize_blp_write(enc_by_slot, slot_count, &dims, first_vis, has_alpha)?;
        Ok(EncoderCtx { bytes })
    }
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

fn header_prefix(heads: &[&[u8]]) -> Vec<u8> {
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
    let mut common_header = header_prefix(&heads);
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

fn tj3_err(handle: raw::tjhandle, key: &'static str) -> BlpError {
    let msg = unsafe {
        let p = raw::tj3GetErrorStr(handle);
        if p.is_null() {
            "unknown".to_string()
        } else {
            CStr::from_ptr(p)
                .to_string_lossy()
                .into_owned()
        }
    };
    BlpError::new(key).with_arg("msg", msg)
}
