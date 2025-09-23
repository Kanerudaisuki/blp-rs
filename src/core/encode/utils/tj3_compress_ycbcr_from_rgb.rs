use crate::error::error::BlpError;

#[allow(dead_code)]
pub fn tj3_compress_ycbcr_from_rgb(
    rgb: &[u8],
    w: usize,
    h: usize,
    pitch: usize, // w*3
    q: i32,
) -> Result<Vec<u8>, BlpError> {
    use std::ffi::CStr;
    use turbojpeg::{libc, raw};

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

        #[inline]
        fn tj3_err(h: raw::tjhandle, key: &'static str) -> BlpError {
            let msg = unsafe {
                let p = raw::tj3GetErrorStr(h);
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

        // качество + 4:4:4 + YCbCr, без optimize
        if raw::tj3Set(hnd, raw::TJPARAM_TJPARAM_QUALITY as i32, q) != 0 {
            return Err(tj3_err(hnd, "tj3.quality"));
        }
        if raw::tj3Set(hnd, raw::TJPARAM_TJPARAM_SUBSAMP as i32, raw::TJSAMP_TJSAMP_444 as i32) != 0 {
            return Err(tj3_err(hnd, "tj3.subsamp"));
        }
        if raw::tj3Set(hnd, raw::TJPARAM_TJPARAM_COLORSPACE as i32, raw::TJCS_TJCS_YCbCr as i32) != 0 {
            return Err(tj3_err(hnd, "tj3.colorspace"));
        }
        if raw::tj3Set(hnd, raw::TJPARAM_TJPARAM_OPTIMIZE as i32, 0) != 0 {
            return Err(tj3_err(hnd, "tj3.optimize"));
        }

        let mut out_ptr: *mut libc::c_uchar = std::ptr::null_mut();
        let mut out_len: raw::size_t = 0;

        // ВХОД — RGB
        if raw::tj3Compress8(
            hnd,
            rgb.as_ptr(), //
            w as libc::c_int,
            pitch as libc::c_int,
            h as libc::c_int,
            raw::TJPF_TJPF_BGR as libc::c_int,
            &mut out_ptr,
            &mut out_len,
        ) != 0
        {
            return Err(tj3_err(hnd, "tj3.compress"));
        }

        let out = std::slice::from_raw_parts(out_ptr, out_len as usize).to_vec();
        raw::tj3Free(out_ptr as *mut libc::c_void);
        Ok(out)
    }
}
