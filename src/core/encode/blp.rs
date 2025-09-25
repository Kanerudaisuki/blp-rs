use crate::core::image::{ImageBlp, MAX_MIPS};
use crate::error::error::BlpError;
use image::RgbaImage;
use std::ffi::CStr;
use std::ptr;
use std::time::Instant;
use turbojpeg::{libc, raw};

// === ДВЕ СТРУКТУРЫ ===

#[derive(Clone)]
pub struct Mip {
    pub w: u32,
    pub h: u32,
    pub vis: bool,
    // техинфа:
    pub encode_ms: f64, // время кодирования мипа (мс)
    // результат:
    pub encoded: Vec<u8>, // очищенный JPEG (header_clean + scan + EOI)
    pub head_len: usize,  // длина header-секции внутри encoded (SOI..end(SOS))
}

pub struct Ctx {
    pub bytes: Vec<u8>,       // Готовый BLP
    pub mips: Vec<Mip>,       // После тримминга; без исходных RgbaImage
    pub has_alpha: bool,      // По первому видимому
    pub encode_ms_total: f64, // Суммарное время кодирования (мс)
}

impl ImageBlp {
    pub fn encode_blp(&self, quality: u8, mip_visible: &[bool]) -> Result<Ctx, BlpError> {
        // --- рабочая структура только для этапа кодирования ---
        struct WorkMip {
            w: u32,
            h: u32,
            vis: bool,
            img: Option<RgbaImage>, // источник; не уходит в итог
        }

        // 1) Собираем рабочие мипы и триммим ведущие «дыры»
        let mut work: Vec<WorkMip> = Vec::with_capacity(self.mipmaps.len().min(MAX_MIPS));
        for (i, m) in self
            .mipmaps
            .iter()
            .take(MAX_MIPS)
            .enumerate()
        {
            let vis = mip_visible
                .get(i)
                .copied()
                .unwrap_or(true);
            work.push(WorkMip { w: m.width, h: m.height, vis, img: m.image.clone() });
        }
        let start_idx = work
            .iter()
            .position(|mm| mm.vis && mm.img.is_some())
            .ok_or_else(|| BlpError::new("no_visible_mips_after_mask"))?;
        let mut work = work.split_off(start_idx);

        // 2) Базовый мип и альфа
        let base_img = work[0].img.as_ref().unwrap();
        if base_img.width() != work[0].w || base_img.height() != work[0].h {
            return Err(BlpError::new("mip.size_mismatch")
                .with_arg("want_w", work[0].w)
                .with_arg("want_h", work[0].h)
                .with_arg("got_w", base_img.width())
                .with_arg("got_h", base_img.height()));
        }
        let has_alpha = base_img.pixels().any(|p| p.0[3] != 255);

        let t0 = Instant::now();
        let mut out_mips: Vec<Mip> = Vec::with_capacity(work.len());
        for wm in &mut work {
            if !(wm.vis && wm.img.is_some()) {
                out_mips.push(Mip { w: wm.w, h: wm.h, vis: wm.vis, encode_ms: 0.0, encoded: Vec::new(), head_len: 0 });
                continue;
            }
            let rgba = wm.img.as_ref().unwrap();
            if rgba.width() != wm.w || rgba.height() != wm.h {
                return Err(BlpError::new("mip.size_mismatch")
                    .with_arg("want_w", wm.w)
                    .with_arg("want_h", wm.h)
                    .with_arg("got_w", rgba.width())
                    .with_arg("got_h", rgba.height()));
            }

            // Упаковка входных пикселей
            let wz = rgba.width() as usize;
            let hz = rgba.height() as usize;
            let src = rgba.as_raw();
            let (packed, pitch) = if has_alpha {
                let mut out = vec![0u8; wz * hz * 4];
                for (dst, px) in out
                    .chunks_exact_mut(4)
                    .zip(src.chunks_exact(4))
                {
                    dst[0] = px[2]; // C ← inv(B)
                    dst[1] = px[1]; // M ← inv(G)
                    dst[2] = px[0]; // Y ← inv(R)
                    dst[3] = px[3]; // K ← inv(A)
                }
                (out, wz * 4)
            } else {
                let mut out = vec![0u8; wz * hz * 3];
                for (dst, px) in out
                    .chunks_exact_mut(3)
                    .zip(src.chunks_exact(4))
                {
                    dst[0] = px[0]; // R
                    dst[1] = px[1]; // G
                    dst[2] = px[2]; // B
                }
                (out, wz * 3)
            };

            let t_mip = Instant::now();

            // TurboJPEG 3 API
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
                if raw::tj3Set(handle, raw::TJPARAM_TJPARAM_COLORSPACE as libc::c_int, if has_alpha { raw::TJCS_TJCS_CMYK as libc::c_int } else { raw::TJCS_TJCS_RGB as libc::c_int }) != 0 {
                    return Err(tj3_err(handle, "tj3.colorspace"));
                }

                let mut out_ptr: *mut libc::c_uchar = ptr::null_mut();
                let mut out_size: raw::size_t = 0;
                let r = raw::tj3Compress8(
                    handle, //
                    packed.as_ptr(),
                    wz as libc::c_int,
                    pitch as libc::c_int,
                    hz as libc::c_int,
                    if has_alpha { raw::TJPF_TJPF_CMYK as libc::c_int } else { raw::TJPF_TJPF_BGR as libc::c_int },
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

            // Санитайз + ре-сборка
            let (head_len, _scan_len) = split_header_and_scan(&jpeg_raw)?;
            let header_clean = rebuild_header_min(&jpeg_raw[..head_len])?;
            let mut rebuilt = Vec::with_capacity(jpeg_raw.len());
            rebuilt.extend_from_slice(&header_clean);
            rebuilt.extend_from_slice(&jpeg_raw[head_len..]); // scan + EOI
            let (h2, _s2) = split_header_and_scan(&rebuilt)?;

            let encode_ms = t_mip.elapsed().as_secs_f64() * 1000.0;
            out_mips.push(Mip { w: wm.w, h: wm.h, vis: wm.vis, encode_ms, encoded: rebuilt, head_len: h2 });
        }

        let encode_ms_total = t0.elapsed().as_secs_f64() * 1000.0;

        // Первый видимый обязан быть закодирован
        if out_mips
            .first()
            .map(|m| m.encoded.is_empty())
            .unwrap_or(true)
        {
            return Err(BlpError::new("first_visible_slot_missing"));
        }

        // 4) Инлайн финализации BLP → bytes
        // общий header как общий префикс
        let mut heads: Vec<&[u8]> = Vec::new();
        for m in &out_mips {
            if !m.encoded.is_empty() {
                heads.push(&m.encoded[..m.head_len]);
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

        #[inline]
        fn write_u32_le_at(buf: &mut Vec<u8>, pos: usize, v: u32) {
            buf[pos..pos + 4].copy_from_slice(&v.to_le_bytes());
        }

        let base_w = out_mips[0].w;
        let base_h = out_mips[0].h;

        let flags: u32 = if has_alpha { 8 } else { 0 };
        let compression: u32 = 0; // JPEG
        let extra_field: u32 = 0;
        let has_mipmaps: u32 = 1;

        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"BLP1");
        bytes.extend_from_slice(&compression.to_le_bytes());
        bytes.extend_from_slice(&flags.to_le_bytes());
        bytes.extend_from_slice(&base_w.to_le_bytes());
        bytes.extend_from_slice(&base_h.to_le_bytes());
        bytes.extend_from_slice(&extra_field.to_le_bytes());
        bytes.extend_from_slice(&has_mipmaps.to_le_bytes());

        // плейсхолдеры для offsets/sizes
        let pos_offsets = bytes.len();
        bytes.resize(bytes.len() + MAX_MIPS * 4, 0);

        let pos_sizes = bytes.len();
        bytes.resize(bytes.len() + MAX_MIPS * 4, 0);

        // общий JPEG header
        let jpeg_header_size: u32 = common_header
            .len()
            .try_into()
            .map_err(|_| BlpError::new("jpeg_header_too_large"))?;

        bytes.extend_from_slice(&jpeg_header_size.to_le_bytes());
        bytes.extend_from_slice(&common_header);
        bytes.extend_from_slice(b"RAFT"); // твой спец-маркер

        // пишем payload’ы и сразу бэкфиллим offset/size
        for i in 0..MAX_MIPS.min(out_mips.len()) {
            let m = &out_mips[i];
            if !m.encoded.is_empty() {
                debug_assert!(m.head_len >= common_header.len());
                debug_assert!(&m.encoded[..common_header.len()] == &common_header[..]);

                let payload = &m.encoded[common_header.len()..];

                let off = bytes.len();
                let sz = payload.len();

                if off > u32::MAX as usize {
                    return Err(BlpError::new("offset_too_large"));
                }
                if sz > u32::MAX as usize {
                    return Err(BlpError::new("payload_too_large"));
                }

                write_u32_le_at(&mut bytes, pos_offsets + (i << 2), off as u32);
                write_u32_le_at(&mut bytes, pos_sizes + (i << 2), sz as u32);

                bytes.extend_from_slice(payload);
            }
        }

        Ok(Ctx {
            bytes, //
            mips: out_mips,
            has_alpha,
            encode_ms_total,
        })
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
