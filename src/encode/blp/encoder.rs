use crate::err::error::BlpError;
use crate::image_blp::{ImageBlp, MAX_MIPS};
use image::RgbaImage;
use image::{DynamicImage, imageops::FilterType};
use std::{ffi::CStr, ptr};
use turbojpeg::{Compressor, Image as TjImage, PixelFormat, Subsamp, libc, raw};

pub struct EncoderCtx {
    pub bytes: Vec<u8>,
}

impl ImageBlp {
    pub fn encode_blp(&self, quality: u8, mip_visible: &[bool]) -> Result<EncoderCtx, BlpError> {
        let mut rgba_cur: RgbaImage = self
            .mipmaps
            .get(0)
            .and_then(|m| m.image.clone())
            .ok_or_else(|| BlpError::new("no_base_image"))?;

        // есть ли альфа
        let mut has_alpha = false;
        for p in rgba_cur.pixels() {
            if p.0[3] != 255 {
                has_alpha = true;
                break;
            }
        }

        // --- строим линейку размеров (делим /2) и маску видимости ---
        let mut dims: Vec<(u32, u32, bool)> = Vec::with_capacity(MAX_MIPS);
        let (mut w, mut h) = (rgba_cur.width(), rgba_cur.height());
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

        // --- первый видимый уровень (BLP не может начинаться с пустого) ---
        let first_vis = dims
            .iter()
            .position(|&(_, _, v)| v)
            .ok_or_else(|| BlpError::new("no_visible_mips_after_mask"))?;

        // --- удобные замыкания ---
        let be_u16 = |b: &[u8]| -> Result<usize, BlpError> {
            if b.len() < 2 {
                return Err(BlpError::new("jpeg.len"));
            }
            Ok(((b[0] as usize) << 8) | b[1] as usize)
        };

        let split_header_and_scan = |jpeg: &[u8]| -> Result<(usize, usize), BlpError> {
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
                    0xD0..=0xD7 | 0x01 => {}
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
                        // найти EOI (FF D9), игнорируя stuffed 00 и RSTn
                        let mut j = head_len;
                        while j + 1 < jpeg.len() {
                            if jpeg[j] == 0xFF {
                                let n = jpeg[j + 1];
                                if n == 0x00 {
                                    j += 2;
                                    continue;
                                }
                                if (0xD0..=0xD7).contains(&n) {
                                    j += 2;
                                    continue;
                                }
                                if n == 0xD9 {
                                    let scan_len = j - head_len;
                                    return Ok((head_len, scan_len));
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
        };

        // зачистка: убрать все APPn (E0..EF) и COM (FE) до SOS; вернуть SOI..SOS
        let strip_app_and_com = |header: &[u8]| -> Result<Vec<u8>, BlpError> {
            if header.len() < 4 || header[0] != 0xFF || header[1] != 0xD8 {
                return Err(BlpError::new("jpeg.sanitize.no_soi"));
            }
            let mut out = Vec::with_capacity(header.len());
            out.extend_from_slice(&header[..2]); // SOI
            let mut i = 2usize;
            loop {
                while i < header.len() && header[i] == 0xFF {
                    i += 1;
                }
                if i >= header.len() {
                    // если SOS не нашли, возвращаем то, что накопили (без APP/COM)
                    return Ok(out);
                }
                let m = header[i];
                i += 1;
                match m {
                    0xDA => {
                        if i + 2 > header.len() {
                            return Err(BlpError::new("jpeg.sanitize.sos_len"));
                        }
                        let seg_len = be_u16(&header[i..i + 2])?;
                        let seg_end = i + seg_len;
                        if seg_end > header.len() {
                            return Err(BlpError::new("jpeg.sanitize.sos_trunc"));
                        }
                        out.extend_from_slice(&[0xFF, 0xDA]);
                        out.extend_from_slice(&header[i..seg_end]);
                        return Ok(out);
                    }
                    0xD0..=0xD7 | 0x01 => {
                        out.extend_from_slice(&[0xFF, m]);
                    }
                    _ => {
                        if i + 2 > header.len() {
                            return Err(BlpError::new("jpeg.sanitize.seg_len"));
                        }
                        let seg_len = be_u16(&header[i..i + 2])?;
                        let seg_start = i - 2;
                        let seg_end = i + seg_len;
                        if seg_end > header.len() {
                            return Err(BlpError::new("jpeg.sanitize.seg_trunc"));
                        }
                        let is_app = (0xE0..=0xEF).contains(&m);
                        let is_com = m == 0xFE;
                        if !(is_app || is_com) {
                            out.extend_from_slice(&header[seg_start..seg_end]);
                        }
                        i = seg_end;
                    }
                }
            }
        };

        // TurboJPEG CMYK (tj3)
        let compress_cmyk = |pixels: &[u8], w: usize, h: usize, pitch: usize, q: i32| -> Result<Vec<u8>, BlpError> {
            unsafe {
                let handle = raw::tj3Init(raw::TJINIT_TJINIT_COMPRESS as libc::c_int);
                if handle.is_null() {
                    return Err(BlpError::new("tj3.init"));
                }
                struct Guard(raw::tjhandle);
                impl Drop for Guard {
                    fn drop(&mut self) {
                        if !self.0.is_null() {
                            unsafe { raw::tj3Destroy(self.0) };
                        }
                    }
                }
                let _g = Guard(handle);

                #[inline]
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
                // QUALITY
                if raw::tj3Set(handle, raw::TJPARAM_TJPARAM_QUALITY as libc::c_int, q) != 0 {
                    return Err(tj3_err(handle, "tj3.quality"));
                }
                // 4:4:4
                if raw::tj3Set(handle, raw::TJPARAM_TJPARAM_SUBSAMP as libc::c_int, raw::TJSAMP_TJSAMP_444 as libc::c_int) != 0 {
                    return Err(tj3_err(handle, "tj3.subsamp"));
                }
                // CMYK
                if raw::tj3Set(handle, raw::TJPARAM_TJPARAM_COLORSPACE as libc::c_int, raw::TJCS_TJCS_CMYK as libc::c_int) != 0 {
                    return Err(tj3_err(handle, "tj3.colorspace"));
                }
                // optimize = 0
                if raw::tj3Set(handle, raw::TJPARAM_TJPARAM_OPTIMIZE as libc::c_int, 0) != 0 {
                    return Err(tj3_err(handle, "tj3.optimize"));
                }

                let mut out_ptr: *mut libc::c_uchar = ptr::null_mut();
                let mut out_size: raw::size_t = 0;
                let r = raw::tj3Compress8(
                    handle,
                    pixels.as_ptr(),
                    w as libc::c_int,
                    pitch as libc::c_int, // bytes/line (stride)
                    h as libc::c_int,
                    raw::TJPF_TJPF_CMYK as libc::c_int,
                    &mut out_ptr,
                    &mut out_size,
                );
                if r != 0 {
                    let msg = {
                        let p = raw::tj3GetErrorStr(handle);
                        if p.is_null() {
                            "unknown".to_string()
                        } else {
                            CStr::from_ptr(p)
                                .to_string_lossy()
                                .into_owned()
                        }
                    };
                    return Err(BlpError::new("tj3.compress").with_arg("msg", msg));
                }
                let slice = std::slice::from_raw_parts(out_ptr, out_size as usize);
                let vec = slice.to_vec();
                raw::tj3Free(out_ptr as *mut libc::c_void);
                Ok(vec)
            }
        };

        // TurboJPEG BGR (high-level)
        let compress_bgr = |pixels: &[u8], w: usize, h: usize, pitch: usize, q: i32| -> Result<Vec<u8>, BlpError> {
            let mut comp = Compressor::new()?; // uses impl From<turbojpeg::Error> for BlpError
            comp.set_quality(q)?;
            comp.set_subsamp(Subsamp::None)?;
            comp.set_optimize(false)?;
            let img = TjImage { pixels, width: w, pitch, height: h, format: PixelFormat::BGR };
            Ok(comp.compress_to_vec(img)?) // here too
        };

        let common_prefix = |heads: &[&[u8]]| -> Vec<u8> {
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
        };

        // --- последовательно дошагиваем по всем уровням, но пишем только от first_vis ---
        struct Slice {
            head_len: usize,
            scan_len: usize,
        }
        struct Enc {
            data: Vec<u8>,
            sl: Slice,
        } // только у видимых уровней

        let mut enc_by_slot: [Option<Enc>; MAX_MIPS] = Default::default(); // slot 0 = уровень first_vis
        let mut slot_count = 0usize;

        for (lvl, &(dw, dh, vis)) in dims.iter().enumerate() {
            // каждый следующий lvl получаем из текущего rgba_cur
            if lvl > 0 {
                let dyn1 = DynamicImage::ImageRgba8(rgba_cur).resize_exact(dw, dh, FilterType::Lanczos3);
                rgba_cur = dyn1.to_rgba8();
            }
            if lvl < first_vis {
                continue;
            } // доходим до первого видимого

            let slot = lvl - first_vis; // сдвинутая нумерация
            if slot >= MAX_MIPS {
                break;
            }
            slot_count = slot_count.max(slot + 1);

            if !vis {
                enc_by_slot[slot] = None; // дырка допустима
                continue;
            }

            // --- кодирование этого уровня ---
            let (wz, hz) = (rgba_cur.width() as usize, rgba_cur.height() as usize);
            let src = rgba_cur.as_raw();

            // упаковать RGBA → BGR/CMYK
            let mut packed = if has_alpha { vec![0u8; wz * hz * 4] } else { vec![0u8; wz * hz * 3] };

            if has_alpha {
                // CMYK: C/M/Y = 255 - B/G/R, K = A
                for (dst, px) in packed
                    .chunks_exact_mut(4)
                    .zip(src.chunks_exact(4))
                {
                    dst[0] = u8::MAX.saturating_sub(px[2]); // C = 255 - B
                    dst[1] = u8::MAX.saturating_sub(px[1]); // M = 255 - G
                    dst[2] = u8::MAX.saturating_sub(px[0]); // Y = 255 - R
                    dst[3] = px[3]; // K = A (если надо инвертировать: u8::MAX.saturating_sub(px[3]))
                }
            } else {
                // BGR (YCbCr)
                for (dst, px) in packed
                    .chunks_exact_mut(3)
                    .zip(src.chunks_exact(4))
                {
                    dst[0] = px[2]; // B
                    dst[1] = px[1]; // G
                    dst[2] = px[0]; // R
                }
            }

            // ВАЖНО: pitch — это bytes/line, а не размер буфера.
            // Можно ставить 0 (плотный буфер) — TurboJPEG возьмёт w*bpp сам.
            let bpp = if has_alpha { 4 } else { 3 };
            let pitch = wz * bpp;

            let jpeg_raw = if has_alpha {
                compress_cmyk(&packed, wz, hz, pitch, quality as i32)?
            } else {
                compress_bgr(&packed, wz, hz, pitch, quality as i32)?
            };

            let (head_len, scan_len) = split_header_and_scan(&jpeg_raw)?;
            let header = &jpeg_raw[..head_len];
            let header_clean = strip_app_and_com(header)?;
            let scan = &jpeg_raw[head_len..head_len + scan_len];
            let eoi = &jpeg_raw[head_len + scan_len..];

            let mut rebuilt = Vec::with_capacity(header_clean.len() + scan.len() + eoi.len());
            rebuilt.extend_from_slice(&header_clean);
            rebuilt.extend_from_slice(scan);
            rebuilt.extend_from_slice(eoi);

            let (h2, s2) = split_header_and_scan(&rebuilt)?;
            enc_by_slot[slot] = Some(Enc { data: rebuilt, sl: Slice { head_len: h2, scan_len: s2 } });
        }

        // гарантируем, что слот 0 присутствует (BLP не стартует с пустого)
        if enc_by_slot[0].is_none() {
            return Err(BlpError::new("first_visible_slot_missing"));
        }

        // --- общий JPEG-хедер по всем существующим слотам ---
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
        // сузить префикс, если какой-то head чуть отличается в конце
        for h in &heads {
            while !h.starts_with(&common_header) && !common_header.is_empty() {
                common_header.pop();
            }
            if !h.starts_with(&common_header) {
                return Err(BlpError::new("head_prefix_mismatch"));
            }
        }

        // --- соберём блоки (по слотам), сохраняя дырки ---
        struct Block<'a> {
            len: u32,
            bytes: &'a [u8],
        }
        let mut blocks: [Option<Block>; MAX_MIPS] = Default::default();

        for i in 0..slot_count {
            if let Some(e) = &enc_by_slot[i] {
                let head = &e.data[..e.sl.head_len];
                let trimmed = &head[common_header.len()..];
                let full_from_trim = &e.data[head.len() - trimmed.len()..];
                blocks[i] = Some(Block { len: full_from_trim.len() as u32, bytes: full_from_trim });
            } else {
                blocks[i] = None; // дырка
            }
        }

        // --- BLP1 шапка и таблицы смещений/длин ---
        let base_w = dims[first_vis].0;
        let base_h = dims[first_vis].1;
        let has_mipmaps: u32 = 1;
        let flags: u32 = if has_alpha { 8 } else { 0 };
        let compression: u32 = 0; // JPEG
        let extra_field: u32 = 5;

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
            } else {
                offsets[i] = 0;
                sizes[i] = 0;
            }
        }

        // --- запись файла ---
        let mut out = Vec::with_capacity(cur as usize);
        out.extend_from_slice(b"BLP1"); // magic
        out.extend_from_slice(&compression.to_le_bytes()); // compression
        out.extend_from_slice(&flags.to_le_bytes()); // flags
        out.extend_from_slice(&base_w.to_le_bytes()); // width (первый видимый)
        out.extend_from_slice(&base_h.to_le_bytes()); // height (первый видимый)
        out.extend_from_slice(&extra_field.to_le_bytes()); // extra=5
        out.extend_from_slice(&has_mipmaps.to_le_bytes()); // hasMipmaps=1
        for &off in &offsets {
            out.extend_from_slice(&off.to_le_bytes());
        }
        for &sz in &sizes {
            out.extend_from_slice(&sz.to_le_bytes());
        }
        // общий JPEG-хедер
        out.extend_from_slice(&(common_header.len() as u32).to_le_bytes());
        out.extend_from_slice(&common_header);
        // payloads
        for i in 0..MAX_MIPS.min(blocks.len()) {
            if let Some(b) = &blocks[i] {
                out.extend_from_slice(b.bytes);
            }
        }

        Ok(EncoderCtx { bytes: out })
    }
}
