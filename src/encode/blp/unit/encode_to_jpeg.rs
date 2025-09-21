// src/encode/blp/unit/encode_to_jpeg
use crate::encode::blp::jpeg::parse::split_header_and_scan;
use crate::encode::blp::unit::unit::MipUnit;
use crate::err::error::BlpError;
use std::time::Instant;
use turbojpeg::{Compressor, Image as TjImage, PixelFormat, Subsamp};

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
            for (dst, px) in scan
                .chunks_exact_mut(4)
                .zip(src.chunks_exact(4))
            {
                // Mirror of decode_jpeg.rs: CMYK stores inverted BGR + alpha in K.
                dst[0] = 255u8.saturating_sub(px[2]); // C  ↔  B channel
                dst[1] = 255u8.saturating_sub(px[1]); // M  ↔  G channel
                dst[2] = 255u8.saturating_sub(px[0]); // Y  ↔  R channel
                dst[3] = 255u8.saturating_sub(px[3]); // K  ↔  A channel
            }
        } else {
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

        let mut compressor = Compressor::new().map_err(|e| BlpError::new("jpeg-compress-init").push_std(e))?;
        compressor
            .set_quality(quality as i32)
            .map_err(|e| BlpError::new("jpeg-set-quality").push_std(e))?;
        // Blizzard assets keep CMYK scans without chroma subsampling.
        compressor
            .set_subsamp(Subsamp::None)
            .map_err(|e| BlpError::new("jpeg-set-subsamp").push_std(e))?;
        compressor
            .set_optimize(false)
            .map_err(|e| BlpError::new("jpeg-set-optimize").push_std(e))?;

        let pitch = (iw as usize) * bpp;
        let format = if has_alpha { PixelFormat::CMYK } else { PixelFormat::BGR };
        let tj_image = TjImage { pixels: scan.as_slice(), width: iw as usize, pitch, height: ih as usize, format };

        let out = compressor
            .compress_to_vec(tj_image)
            .map_err(|e| BlpError::new("jpeg-compress").push_std(e))?;

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
