use crate::core::image::ImageBlp;
use crate::error::error::BlpError;
use image::{Rgba, RgbaImage};
use jpeg_decoder::{Decoder, PixelFormat};
use std::io::Cursor;

impl ImageBlp {
    /// JPEG — берём общий header из header_offset/header_length, для каждого мипа склеиваем [header][tail] и декодим.
    pub(crate) fn decode_jpeg(&mut self, buf: &[u8]) -> Result<(), BlpError> {
        let h_off = self.header_offset;
        let h_len = self.header_length;
        if h_off.checked_add(h_len).is_none() || h_off + h_len > buf.len() {
            return Err(BlpError::new("jpeg.header.oob"));
        }
        let header_bytes = &buf[h_off..h_off + h_len];

        for i in 0..self.mipmaps.len() {
            let off = self.mipmap_offsets[i] as usize;
            let len = self.mipmap_lengths[i] as usize;
            if len == 0 {
                continue;
            }
            if off.checked_add(len).is_none() || off + len > buf.len() {
                continue;
            }

            let tail = &buf[off..off + len];

            let mut full = Vec::with_capacity(header_bytes.len() + tail.len());
            full.extend_from_slice(header_bytes);
            full.extend_from_slice(tail);

            // декодируем JPEG
            let mut dec = Decoder::new(Cursor::new(&full));
            dec.read_info().map_err(|e| {
                BlpError::from(e)
                    .with_arg("phase", "read_info")
                    .with_arg("mip", i as u32)
            })?;
            let info = dec
                .info()
                .ok_or_else(|| BlpError::new("jpeg.meta.missing").with_arg("mip", i as u32))?;
            let (w, h) = (info.width as u32, info.height as u32);
            let pixels = dec.decode().map_err(|e| {
                BlpError::from(e)
                    .with_arg("phase", "decode")
                    .with_arg("mip", i as u32)
            })?;

            let mut img = RgbaImage::new(w, h);
            let force_opaque = self.alpha_bits == 0;

            println!("info.pixel_format {:?}", info.pixel_format);

            match info.pixel_format {
                PixelFormat::CMYK32 => {
                    if pixels.len() != (w as usize * h as usize * 4) {
                        return Err(BlpError::new("jpeg.size.mismatch")
                            .with_arg("fmt", "CMYK32")
                            .with_arg("mip", i as u32));
                    }
                    for (p, px) in img.pixels_mut().enumerate() {
                        let idx = p * 4;
                        let c = pixels[idx + 0];
                        let m = pixels[idx + 1];
                        let y = pixels[idx + 2];
                        let k = pixels[idx + 3];
                        let a = if force_opaque { 255 } else { 255u8.saturating_sub(k) };
                        *px = Rgba([255u8.saturating_sub(y), 255u8.saturating_sub(m), 255u8.saturating_sub(c), a]);
                    }
                }
                PixelFormat::RGB24 => {
                    if pixels.len() != (w as usize * h as usize * 3) {
                        return Err(BlpError::new("jpeg.size.mismatch")
                            .with_arg("fmt", "RGB24")
                            .with_arg("mip", i as u32));
                    }

                    if option_env!("NEVER").is_some() {
                        for (p, px) in img.pixels_mut().enumerate() {
                            let idx = p * 3;
                            *px = Rgba([
                                pixels[idx + 2], //
                                pixels[idx + 1],
                                pixels[idx + 0],
                                255,
                            ]);
                        }
                    } else {
                        for (p, px) in img.pixels_mut().enumerate() {
                            let idx = p * 3;
                            let r = pixels[idx + 0] as f32;
                            let g = pixels[idx + 1] as f32;
                            let b = pixels[idx + 2] as f32;

                            // RGB → YCbCr (BT.601 JPEG)
                            let y = (0.2990 * r + 0.5870 * g + 0.1140 * b).round();
                            let cb = (128.0 - 0.168736 * r - 0.331264 * g + 0.5 * b).round();
                            let cr = (128.0 + 0.5 * r - 0.418688 * g - 0.081312 * b).round();

                            // зажимаем в [0,255]
                            let y = y.clamp(0.0, 255.0) as u8;
                            let cb = cb.clamp(0.0, 255.0) as u8;
                            let cr = cr.clamp(0.0, 255.0) as u8;

                            // кладём Y,Cb,Cr как RGB (вот и получается «пиздец»)
                            *px = Rgba([y, cb, cr, 255]);
                        }
                    }
                }
                PixelFormat::L8 => {
                    if pixels.len() != (w as usize * h as usize) {
                        return Err(BlpError::new("jpeg.size.mismatch")
                            .with_arg("fmt", "L8")
                            .with_arg("mip", i as u32));
                    }
                    for (p, px) in img.pixels_mut().enumerate() {
                        let l = pixels[p];
                        *px = Rgba([l, l, l, 255]);
                    }
                }
                PixelFormat::L16 => {
                    if pixels.len() != (w as usize * h as usize * 2) {
                        return Err(BlpError::new("jpeg.size.mismatch")
                            .with_arg("fmt", "L16")
                            .with_arg("mip", i as u32));
                    }
                    for (chunk, px) in pixels
                        .chunks_exact(2)
                        .zip(img.pixels_mut())
                    {
                        let l16 = u16::from_be_bytes([chunk[0], chunk[1]]);
                        let l8 = (l16 / 257) as u8;
                        *px = Rgba([l8, l8, l8, 255]);
                    }
                }
            }

            // кладём в совпадающий по размеру уровень, не пересоздавая Mipmap
            if self.mipmaps[i].width == w && self.mipmaps[i].height == h {
                self.mipmaps[i].image = Some(img);
            } else if let Some(level) = (0..self.mipmaps.len()).find(|&lvl| self.mipmaps[lvl].width == w && self.mipmaps[lvl].height == h) {
                self.mipmaps[level].image = Some(img);
            }
        }
        Ok(())
    }
}
