use image::{Rgba, RgbaImage};
use jpeg_decoder::Decoder;
use std::io::Cursor;

#[derive(Debug)]
pub struct Mipmap {
    pub width: u32,
    pub height: u32,
    pub image: Option<RgbaImage>,
}

impl Mipmap {
    pub fn read_jpeg(jpeg_header_chunk: &[u8], jpeg_chunk: &[u8]) -> Result<Mipmap, Box<dyn std::error::Error + Send + Sync>> {
        let mut full_jpeg = Vec::with_capacity(jpeg_header_chunk.len() + jpeg_chunk.len());
        full_jpeg.extend_from_slice(jpeg_header_chunk);
        full_jpeg.extend_from_slice(jpeg_chunk);

        let mut decoder = Decoder::new(Cursor::new(&full_jpeg));
        decoder.read_info()?; // если не прочиталось — ошибка

        let metadata = decoder
            .info()
            .ok_or("No JPEG metadata after read_info")?;
        let dec_w = metadata.width as u32;
        let dec_h = metadata.height as u32;

        let pixels = decoder.decode()?; // jpeg_decoder возвращает Vec<u8> или ошибку
        if pixels.len() != (dec_w * dec_h * 4) as usize {
            return Err("JPEG pixel buffer size mismatch".into());
        }

        let mut image = RgbaImage::new(dec_w, dec_h);
        for (i, pixel) in image.pixels_mut().enumerate() {
            let idx = i * 4;
            *pixel = Rgba([255u8.saturating_sub(pixels[idx + 2]), 255u8.saturating_sub(pixels[idx + 1]), 255u8.saturating_sub(pixels[idx + 0]), 255u8.saturating_sub(pixels[idx + 3])]);
        }

        Ok(Mipmap { width: dec_w, height: dec_h, image: Some(image) })
    }

    pub fn read_direct<R: std::io::Read>(reader: &mut R, width: u32, height: u32, palette: &[[u8; 3]; 256], alpha_bits: u32) -> Result<Mipmap, Box<dyn std::error::Error + Send + Sync>> {
        let pixel_count = (width * height) as usize;

        let mut indices = vec![0u8; pixel_count];
        reader.read_exact(&mut indices)?;

        // Альфа-канал
        let alpha_bytes = match alpha_bits {
            0 => 0,
            1 => (pixel_count + 7) / 8,
            4 => (pixel_count + 1) / 2,
            8 => pixel_count,
            _ => return Err("Unsupported alpha_bits".into()),
        };
        let mut alpha_raw = vec![0u8; alpha_bytes];
        if alpha_bytes > 0 {
            reader.read_exact(&mut alpha_raw)?;
        }

        // Собираем RGBA
        let mut image = RgbaImage::new(width, height);
        for i in 0..pixel_count {
            let idx = indices[i] as usize;
            let [r, g, b] = palette[idx];
            let a = match alpha_bits {
                0 => 255,
                1 => {
                    let byte = alpha_raw[i / 8];
                    let bit = (byte >> (i % 8)) & 1;
                    if bit == 1 { 255 } else { 0 }
                }
                4 => {
                    let byte = alpha_raw[i / 2];
                    let nibble = if i % 2 == 0 { byte & 0x0F } else { byte >> 4 };
                    (nibble << 4) | nibble // чтобы растянуть до 8 бит
                }
                8 => alpha_raw[i],
                _ => 255,
            };
            image
                .get_pixel_mut((i as u32) % width, (i as u32) / width)
                .0 = [r, g, b, a];
        }

        Ok(Mipmap { width, height, image: Some(image) })
    }
}
