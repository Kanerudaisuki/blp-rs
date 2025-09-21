use crate::encode::blp::jpeg::build::build_sof0_sos_from_templates;
use crate::encode::blp::jpeg::types::{PlanTemplate, extract_plan_template_from_common_header};
use crate::err::error::BlpError;
use crate::header::Header;
use crate::image_blp::{ImageBlp, MAX_MIPS};
use crate::mipmap::Mipmap;
use byteorder::{LittleEndian, ReadBytesExt};
use image::{Rgba, RgbaImage};
use jpeg_decoder::{Decoder, PixelFormat};
use std::io::{Cursor, Read};

impl ImageBlp {
    pub(crate) fn decode_jpeg(cursor: &mut Cursor<&[u8]>, header: &Header, slices: Vec<Option<&[u8]>>, mipmaps: &mut Vec<Mipmap>) -> Result<(), BlpError> {
        let jpeg_header_size = cursor.read_u32::<LittleEndian>()? as usize;
        let mut jpeg_header_chunk = vec![0u8; jpeg_header_size];
        cursor.read_exact(&mut jpeg_header_chunk)?;

        let plan_template = match extract_plan_template_from_common_header(&jpeg_header_chunk) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[decode] failed to read JPEG plan from common header: {e}");
                None
            }
        };

        for (idx, slice_opt) in slices.into_iter().enumerate() {
            if let Some(slice) = slice_opt {
                let expected_width = (header.width >> idx).max(1);
                let expected_height = (header.height >> idx).max(1);
                let mipmap = Mipmap::decode_jpeg_inner(header, &jpeg_header_chunk, plan_template.as_ref(), slice, expected_width, expected_height).map_err(|e| {
                    e.ctx("decode.jpeg.mip")
                        .with_arg("mip", idx as u32)
                        .with_arg("header_len", jpeg_header_chunk.len() as u32)
                        .with_arg("slice_len", slice.len() as u32)
                })?;

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
    pub fn decode_jpeg_inner(header: &Header, jpeg_header_chunk: &[u8], plan_template: Option<&PlanTemplate>, jpeg_chunk: &[u8], expected_width: u32, expected_height: u32) -> Result<Mipmap, BlpError> {
        let mut full = Vec::with_capacity(jpeg_header_chunk.len() + jpeg_chunk.len() + if plan_template.is_some() { 32 } else { 0 });
        full.extend_from_slice(jpeg_header_chunk);
        if let Some(tpl) = plan_template {
            let sof0_sos = build_sof0_sos_from_templates(&tpl.sof0, &tpl.sos, expected_width as u16, expected_height as u16)?;
            full.extend_from_slice(&sof0_sos);
            full.extend_from_slice(jpeg_chunk);
            full.extend_from_slice(&[0xFF, 0xD9]);
        } else {
            full.extend_from_slice(jpeg_chunk);
        }

        let mut dec = Decoder::new(Cursor::new(&full));
        dec.read_info()
            .map_err(|e| BlpError::from(e).with_arg("phase", "read_info"))?;
        let info = dec
            .info()
            .ok_or_else(|| BlpError::new("jpeg-metadata-missing").with_arg("msg", "No JPEG metadata after read_info"))?;

        let (w, h) = (info.width as u32, info.height as u32);

        let pixels = dec
            .decode()
            .map_err(|e| BlpError::from(e).with_arg("phase", "decode"))?;
        let mut img = RgbaImage::new(w, h);

        let force_opaque = header.alpha_bits == 0;

        match info.pixel_format {
            // 4 компонента, классический для BLP/JPEG вариант: CMYK
            // → RGB = 255 - (Y,M,C), A = (alpha_bits==0)?255:(255-K)
            PixelFormat::CMYK32 => {
                if pixels.len() != (w * h * 4) as usize {
                    return Err(BlpError::new("jpeg-metadata-missing").with_arg("msg", "JPEG buffer size mismatch for CMYK32"));
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
                    return Err(BlpError::new("jpeg-metadata-missing").with_arg("msg", "JPEG buffer size mismatch for RGB24"));
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
                    return Err(BlpError::new("jpeg-metadata-missing").with_arg("msg", "JPEG buffer size mismatch for L8"));
                }
                for (i, px) in img.pixels_mut().enumerate() {
                    let l = pixels[i];
                    *px = Rgba([l, l, l, 255]);
                }
            }
            PixelFormat::L16 => {
                // Ожидаем по 2 байта на пиксель
                if pixels.len() != (w * h * 2) as usize {
                    return Err(BlpError::new("jpeg-metadata-missing").with_arg("msg", "Buffer size mismatch for L16"));
                }

                // PNG/TIFF обычно хранят L16 в big-endian (MSB, затем LSB).
                // Если твой источник little-endian, замени from_be_bytes на from_le_bytes.
                for (chunk, px) in pixels
                    .chunks_exact(2)
                    .zip(img.pixels_mut())
                {
                    let l16 = u16::from_be_bytes([chunk[0], chunk[1]]);
                    // Равномерная свёртка 16-бит в 8-бит: 0..65535 -> 0..255
                    let l8 = (l16 / 257) as u8;
                    *px = Rgba([l8, l8, l8, 255]);
                }
            }
        }

        Ok(Mipmap { width: w, height: h, image: Some(img) })
    }
}
