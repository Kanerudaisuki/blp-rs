use crate::encode::blp::unit::unit::MipUnit;
use crate::err::error::BlpError;
use mozjpeg::{ColorSpace, Compress};
use std::time::Instant;

impl MipUnit {
    pub fn encode_to_jpeg(&mut self, rgba: &image::RgbaImage, has_alpha: bool, quality: u8) -> Result<(), BlpError> {
        let (iw, ih) = (rgba.width(), rgba.height());
        if self.width != 0 || self.height != 0 {
            self.jpeg_full.clear();
            self.jpeg_full_bytes = 0;
            self.encode_ms_acc = 0.0;
            self.skip_reason = None;
        }
        self.width = iw;
        self.height = ih;

        let src = rgba.as_raw(); // RGBA RGBA ...
        debug_assert_eq!(src.len(), (iw as usize) * (ih as usize) * 4);

        // 3 BPP (BGR) без альфы, 4 BPP (CMYK: K=A) с альфой
        let bpp = if has_alpha { 4 } else { 3 };
        let mut scan = vec![0u8; (iw as usize) * (ih as usize) * bpp];

        // Единый цикл: B,G,R всегда, K добавляем только если has_alpha
        let mut di = 0usize;
        for px in src.chunks_exact(4) {
            let r = px[0];
            let g = px[1];
            let b = px[2];
            let a = px[3];

            // BGR всегда
            scan[di] = b;
            scan[di + 1] = g;
            scan[di + 2] = r;
            di += 3;

            // K = A только если есть альфа
            if has_alpha {
                scan[di] = a;
                di += 1;
            }
        }

        let mut out = Vec::new();
        let t0 = Instant::now();

        // Выбор цветового пространства и stride
        let mut comp = Compress::new(if has_alpha {
            ColorSpace::JCS_CMYK // интерпретируем [B,G,R,A] как [C,M,Y,K]
        } else {
            ColorSpace::JCS_EXT_BGR // интерпретируем [B,G,R]
        });
        comp.set_size(iw as usize, ih as usize);
        comp.set_quality(quality as f32);

        let mut writer = comp
            .start_compress(&mut out)
            .map_err(|e| BlpError::new("third-party-error").push_std(e))?;

        let stride = (iw as usize) * bpp;
        for y in 0..(ih as usize) {
            let row = &scan[y * stride..(y + 1) * stride];
            writer
                .write_scanlines(row)
                .map_err(|e| BlpError::new("third-party-error").push_std(e))?;
        }
        writer
            .finish()
            .map_err(|e| BlpError::new("third-party-error").push_std(e))?;

        if out.is_empty() {
            return Err(BlpError::new("third-party-error"));
        }

        self.jpeg_full = out;
        self.jpeg_full_bytes = self.jpeg_full.len();
        self.encode_ms_acc = t0.elapsed().as_secs_f64() * 1000.0;
        self.included = true;
        self.skip_reason = None;

        Ok(())
    }
}
