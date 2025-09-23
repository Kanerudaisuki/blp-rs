use crate::core::encode::utils::tj3_err::tj3_err;
use crate::error::error::BlpError;
use turbojpeg::{libc, raw};

pub fn tj3_compress_ycbcr_from_bgr(
    bgr: &[u8],
    w: usize,
    h: usize,
    pitch: usize, // w*3
    q: i32,
) -> Result<Vec<u8>, BlpError> {
    unsafe {
        let hnd = raw::tj3Init(raw::TJINIT_TJINIT_COMPRESS as libc::c_int);
        if hnd.is_null() {
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
        let _g = Guard(hnd);

        // качество
        if raw::tj3Set(hnd, raw::TJPARAM_TJPARAM_QUALITY as i32, q) != 0 {
            return Err(tj3_err(hnd, "tj3.quality"));
        }
        // 4:4:4
        if raw::tj3Set(hnd, raw::TJPARAM_TJPARAM_SUBSAMP as i32, raw::TJSAMP_TJSAMP_444 as i32) != 0 {
            return Err(tj3_err(hnd, "tj3.subsamp"));
        }
        // Целевой colorspace — YCbCr
        if raw::tj3Set(hnd, raw::TJPARAM_TJPARAM_COLORSPACE as i32, raw::TJCS_TJCS_YCbCr as i32) != 0 {
            return Err(tj3_err(hnd, "tj3.colorspace"));
        }
        // без оптимизации
        if raw::tj3Set(hnd, raw::TJPARAM_TJPARAM_OPTIMIZE as i32, 0) != 0 {
            return Err(tj3_err(hnd, "tj3.optimize"));
        }

        let mut out_ptr: *mut libc::c_uchar = std::ptr::null_mut();
        let mut out_len: raw::size_t = 0;

        // Входной формат — BGR
        let rc = raw::tj3Compress8(
            hnd,
            bgr.as_ptr(),
            w as libc::c_int,
            pitch as libc::c_int,
            h as libc::c_int,
            raw::TJPF_TJPF_BGR as libc::c_int, // BGR input
            &mut out_ptr,
            &mut out_len,
        );
        if rc != 0 {
            return Err(tj3_err(hnd, "tj3.compress"));
        }

        let out = std::slice::from_raw_parts(out_ptr, out_len as usize).to_vec();
        raw::tj3Free(out_ptr as *mut libc::c_void);
        Ok(out)
    }
}
