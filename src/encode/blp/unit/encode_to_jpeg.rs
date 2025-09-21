// src/encode/blp/unit/encode_to_jpeg
use crate::encode::blp::jpeg::parse::split_header_and_scan;
use crate::encode::blp::unit::unit::MipUnit;
use crate::err::error::BlpError;
use std::ffi::CStr;
use std::ptr;
use std::time::Instant;
use turbojpeg::libc;
use turbojpeg::{Compressor, Image as TjImage, PixelFormat, Subsamp, raw};

impl MipUnit {
    /// Baseline JPEG:
    /// - без альфы: вход BGR (3 BPP), файл → YCbCr 4:4:4
    /// - с альфой: вход CMYK (C=B, M=G, Y=R, K=A) (4 BPP), файл → CMYK
    /// - отключены оптимизации → стабильные DQT/DHT между мипами
    /// После кодирования сразу вычисляем `jpeg_slices` (head_len, scan_len).
    pub fn encode_to_jpeg(&mut self, rgba: &image::RgbaImage, has_alpha: bool, quality: u8) -> Result<(), BlpError> {
        let (iw, ih) = (rgba.width(), rgba.height());

        if self.width != 0 || self.height != 0 {
            self.jpeg_full.clear();
            self.jpeg_full_bytes = 0;
            self.jpeg_slices = None;
            self.encode_ms_acc = 0.0;
            self.skip_reason = None;
        }
        self.width = iw;
        self.height = ih;

        // pack RGBA -> BGR / CMYK (C=B, M=G, Y=R, K=A)
        let src = rgba.as_raw();
        let bpp = if has_alpha { 4 } else { 3 };
        let mut scan = vec![0u8; (iw as usize) * (ih as usize) * bpp];

        if has_alpha {
            // RGBA → CMYK без инверсий (C = B, M = G, Y = R, K = A)
            for (dst, px) in scan
                .chunks_exact_mut(4)
                .zip(src.chunks_exact(4))
            {
                dst[0] = px[2]; // C ← B
                dst[1] = px[1]; // M ← G
                dst[2] = px[0]; // Y ← R
                dst[3] = px[3]; // K ← A
            }
        } else {
            // BGR без инверсий
            for (dst, px) in scan
                .chunks_exact_mut(3)
                .zip(src.chunks_exact(4))
            {
                dst[0] = px[2]; // B
                dst[1] = px[1]; // G
                dst[2] = px[0]; // R
            }
        }

        let t0 = Instant::now();

        let pitch = (iw as usize) * bpp;
        let out = if has_alpha {
            compress_cmyk_jpeg(scan.as_slice(), iw as usize, ih as usize, pitch, quality as i32)?
        } else {
            let mut compressor = Compressor::new().map_err(|e| BlpError::new("jpeg-compress-init").push_std(e))?;
            compressor
                .set_quality(quality as i32)
                .map_err(|e| BlpError::new("jpeg-set-quality").push_std(e))?;
            compressor
                .set_subsamp(Subsamp::None)
                .map_err(|e| BlpError::new("jpeg-set-subsamp").push_std(e))?;
            compressor
                .set_optimize(false)
                .map_err(|e| BlpError::new("jpeg-set-optimize").push_std(e))?;
            let tj_image = TjImage { pixels: scan.as_slice(), width: iw as usize, pitch, height: ih as usize, format: PixelFormat::BGR };
            compressor
                .compress_to_vec(tj_image)
                .map_err(|e| BlpError::new("jpeg-compress").push_std(e))?
        };

        // тут же считаем head_len/scan_len — будем писать только scan в контейнер
        let slices = split_header_and_scan(&out).map_err(|e| {
            e.ctx("encode.split_header_and_scan")
                .with_arg("mip", self.index as u32)
                .with_arg("jpeg_len", out.len() as u32)
        })?;

        self.jpeg_full = out;
        self.jpeg_full_bytes = self.jpeg_full.len();
        self.jpeg_slices = Some(slices);
        self.encode_ms_acc = t0.elapsed().as_secs_f64() * 1000.0;
        self.included = true;
        self.skip_reason = None;

        Ok(())
    }
}

fn compress_cmyk_jpeg(pixels: &[u8], width: usize, height: usize, pitch: usize, quality: i32) -> Result<Vec<u8>, BlpError> {
    unsafe {
        let handle = raw::tj3Init(raw::TJINIT_TJINIT_COMPRESS as libc::c_int);
        if handle.is_null() {
            return Err(tj_error(ptr::null_mut(), "jpeg-compress-init"));
        }
        let _guard = TjHandleGuard { handle };

        tj_set(handle, raw::TJPARAM_TJPARAM_QUALITY, quality as libc::c_int, "jpeg-set-quality")?;
        tj_set(handle, raw::TJPARAM_TJPARAM_SUBSAMP, raw::TJSAMP_TJSAMP_444 as libc::c_int, "jpeg-set-subsamp")?;
        tj_set(handle, raw::TJPARAM_TJPARAM_COLORSPACE, raw::TJCS_TJCS_YCCK as libc::c_int, "jpeg-set-colorspace")?;
        tj_set(handle, raw::TJPARAM_TJPARAM_OPTIMIZE, 0, "jpeg-set-optimize")?;

        let mut out_ptr: *mut libc::c_uchar = ptr::null_mut();
        let mut out_size: raw::size_t = 0;
        let res = raw::tj3Compress8(handle, pixels.as_ptr(), width as libc::c_int, pitch as libc::c_int, height as libc::c_int, raw::TJPF_TJPF_CMYK as libc::c_int, &mut out_ptr, &mut out_size);
        if res != 0 {
            return Err(tj_error(handle, "jpeg-compress"));
        }

        let slice = std::slice::from_raw_parts(out_ptr, out_size as usize);
        let vec = slice.to_vec();
        raw::tj3Free(out_ptr as *mut libc::c_void);
        Ok(vec)
    }
}

struct TjHandleGuard {
    handle: raw::tjhandle,
}

impl Drop for TjHandleGuard {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { raw::tj3Destroy(self.handle) };
            self.handle = ptr::null_mut();
        }
    }
}

fn tj_set(handle: raw::tjhandle, param: raw::TJPARAM, value: libc::c_int, key: &'static str) -> Result<(), BlpError> {
    let res = unsafe { raw::tj3Set(handle, param as libc::c_int, value) };
    if res != 0 { Err(tj_error(handle, key)) } else { Ok(()) }
}

fn tj_error(handle: raw::tjhandle, key: &'static str) -> BlpError {
    let msg = unsafe {
        let ptr = raw::tj3GetErrorStr(handle);
        if ptr.is_null() {
            "unknown".to_string()
        } else {
            CStr::from_ptr(ptr)
                .to_string_lossy()
                .into_owned()
        }
    };
    BlpError::new(key).with_arg("msg", msg)
}
