use crate::header::Header;
use crate::image_blp::{ImageBlp, MAX_MIPS};
use crate::mipmap::Mipmap;
use byteorder::{LittleEndian, ReadBytesExt};
use image::{Rgba, RgbaImage};
use jpeg_decoder::{Decoder, PixelFormat};
use std::error::Error;
use std::io::{Cursor, Read};

impl ImageBlp {
    pub(crate) fn decode_jpeg(cursor: &mut Cursor<&[u8]>, header: &Header, slices: Vec<Option<&[u8]>>, mipmaps: &mut Vec<Mipmap>) -> Result<(), Box<dyn Error + Send + Sync>> {
        let jpeg_header_size = cursor.read_u32::<LittleEndian>()? as usize;
        let mut jpeg_header_chunk = vec![0u8; jpeg_header_size];
        cursor.read_exact(&mut jpeg_header_chunk)?;

        for (_, slice_opt) in slices.into_iter().enumerate() {
            if let Some(slice) = slice_opt {
                let mipmap = Mipmap::decode_jpeg_inner(header, &jpeg_header_chunk, slice)?;

                // тот же подбор уровня
                let mut level = 0;
                let mut w = header.width;
                let mut h = header.height;
                while level < MAX_MIPS {
                    if w.max(1) == mipmap.width && h.max(1) == mipmap.height {
                        break;
                    }
                    if w == 0 && h == 0 {
                        break;
                    }
                    w >>= 1;
                    h >>= 1;
                    level += 1;
                }
                if level < MAX_MIPS {
                    mipmaps[level] = mipmap;
                }
            }
        }
        Ok(())
    }
}

impl Mipmap {
    pub fn decode_jpeg_inner(header: &Header, jpeg_header_chunk: &[u8], jpeg_chunk: &[u8]) -> Result<Mipmap, Box<dyn Error + Send + Sync>> {
        let mut full = Vec::with_capacity(jpeg_header_chunk.len() + jpeg_chunk.len());
        full.extend_from_slice(jpeg_header_chunk);
        full.extend_from_slice(jpeg_chunk);

        let mut dec = Decoder::new(Cursor::new(&full));
        dec.read_info()?;
        let info = dec
            .info()
            .ok_or("No JPEG metadata after read_info")?;
        let (w, h) = (info.width as u32, info.height as u32);

        let pixels = dec.decode()?;
        let mut img = RgbaImage::new(w, h);

        let force_opaque = header.alpha_bits == 0;

        match info.pixel_format {
            // 4 компонента, классический для BLP/JPEG вариант: CMYK
            // → RGB = 255 - (Y,M,C), A = (alpha_bits==0)?255:(255-K)
            PixelFormat::CMYK32 => {
                if pixels.len() != (w * h * 4) as usize {
                    return Err("JPEG buffer size mismatch for CMYK32".into());
                }
                for (i, px) in img.pixels_mut().enumerate() {
                    let idx = i * 4;
                    let c = pixels[idx + 0];
                    let m = pixels[idx + 1];
                    let y = pixels[idx + 2];
                    let k = pixels[idx + 3];
                    let a = if force_opaque { 255 } else { 255u8.saturating_sub(k) };
                    *px = Rgba([
                        255u8.saturating_sub(y), // R
                        255u8.saturating_sub(m), // G
                        255u8.saturating_sub(c), // B
                        a,                       // A
                    ]);
                }
            }

            // 3 компонента — на всякий случай: просто RGB, альфы в JPEG нет
            PixelFormat::RGB24 => {
                if pixels.len() != (w * h * 3) as usize {
                    return Err("JPEG buffer size mismatch for RGB24".into());
                }
                for (i, px) in img.pixels_mut().enumerate() {
                    let idx = i * 3;
                    let r = pixels[idx + 0];
                    let g = pixels[idx + 1];
                    let b = pixels[idx + 2];
                    *px = Rgba([r, g, b, 255]); // alpha_bits==0 ⇒ 255; если !=0 — всё равно 255 (в JPEG её нет)
                }
            }

            // серый и пр. — сведём к L8
            PixelFormat::L8 => {
                if pixels.len() != (w * h) as usize {
                    return Err("JPEG buffer size mismatch for L8".into());
                }
                for (i, px) in img.pixels_mut().enumerate() {
                    let l = pixels[i];
                    *px = Rgba([l, l, l, 255]);
                }
            }
            PixelFormat::L16 => todo!(),
        }

        Ok(Mipmap { width: w, height: h, image: Some(img) })
    }
}
