use crate::core::encode::blp::tj3_base_config;
use crate::core::encode::utils::tj3_err::tj3_err;
use crate::error::error::BlpError;
use std::ptr;
use turbojpeg::{libc, raw};

pub fn tj3_compress_cmyk(pixels: &[u8], w: usize, h: usize, pitch: usize, q: i32) -> Result<Vec<u8>, BlpError> {
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

        tj3_base_config(handle, q)?;
        if raw::tj3Set(handle, raw::TJPARAM_TJPARAM_COLORSPACE as libc::c_int, raw::TJCS_TJCS_CMYK as libc::c_int) != 0 {
            return Err(tj3_err(handle, "tj3.colorspace"));
        }

        let mut out_ptr: *mut libc::c_uchar = ptr::null_mut();
        let mut out_size: raw::size_t = 0;
        let r = raw::tj3Compress8(
            handle,
            pixels.as_ptr(),
            w as libc::c_int,
            pitch as libc::c_int,
            h as libc::c_int,
            raw::TJPF_TJPF_CMYK as libc::c_int, // вход: CMYK
            &mut out_ptr,
            &mut out_size,
        );
        if r != 0 {
            return Err(tj3_err(handle, "tj3.compress"));
        }
        let slice = std::slice::from_raw_parts(out_ptr, out_size as usize);
        let vec = slice.to_vec();
        raw::tj3Free(out_ptr as *mut libc::c_void);
        Ok(vec)
    }
}
