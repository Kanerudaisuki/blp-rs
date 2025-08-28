use crate::header::Header;
use crate::image_blp::ImageBlp;
use crate::mipmap::Mipmap;
use byteorder::{LittleEndian, ReadBytesExt};
use image::{Rgba, RgbaImage};
use jpeg_decoder::Decoder;
use std::error::Error;
use std::io::{Cursor, Read};

impl ImageBlp {
    pub(crate) fn decode_jpeg(
        cursor: &mut Cursor<&[u8]>, //
        header: &Header,
        slices: Vec<Option<&[u8]>>,
        mipmaps: &mut Vec<Mipmap>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let jpeg_header_size = cursor.read_u32::<LittleEndian>()? as usize;
        let mut jpeg_header_chunk = vec![0u8; jpeg_header_size];
        cursor.read_exact(&mut jpeg_header_chunk)?;

        for (_, slice_opt) in slices.into_iter().enumerate() {
            if let Some(slice) = slice_opt {
                let mipmap = Mipmap::decode_jpeg(&jpeg_header_chunk, slice)?;

                let mut level = 0;
                let mut w = header.width;
                let mut h = header.height;
                while level < 16 {
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

                if level < 16 {
                    mipmaps[level] = mipmap;
                }
            }
        }

        Ok(())
    }
}

impl Mipmap {
    pub fn decode_jpeg(
        jpeg_header_chunk: &[u8], //
        jpeg_chunk: &[u8],
    ) -> Result<Mipmap, Box<dyn Error + Send + Sync>> {
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
            *pixel = Rgba([
                255u8.saturating_sub(pixels[idx + 2]), //
                255u8.saturating_sub(pixels[idx + 1]),
                255u8.saturating_sub(pixels[idx + 0]),
                255u8.saturating_sub(pixels[idx + 3]),
            ]);
        }

        Ok(Mipmap { width: dec_w, height: dec_h, image: Some(image) })
    }
}
