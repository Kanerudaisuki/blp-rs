use crate::header::{HEADER_SIZE, Header};
use crate::texture_type::TextureType;
use byteorder::{LittleEndian, ReadBytesExt};
use image::{Rgba, RgbaImage};
use jpeg_decoder::Decoder as JpegDecoder;
use std::error::Error;
use std::io::{Cursor, Read};
use crate::mipmap::Mipmap;

#[derive(Debug)]
pub struct ImageBlp {
    pub header: Header,
    pub mipmaps: Vec<Mipmap>,
}

impl ImageBlp {
    pub fn from_bytes(buf: &[u8]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let mut cursor = Cursor::new(buf);
        let header = Header::parse(&mut cursor)?;

        if header.texture_type != TextureType::JPEG {
            return Err("Only JPEG-encoded BLP is supported".into());
        }

        if cursor.position() != HEADER_SIZE {
            eprintln!("Warning: unexpected cursor position, got {}", cursor.position());
        }

        let jpeg_header_size = cursor.read_u32::<LittleEndian>()? as usize;
        let mut jpeg_header_chunk = vec![0u8; jpeg_header_size];
        cursor.read_exact(&mut jpeg_header_chunk)?;

        let mut mipmaps = Vec::new();
        let mut width = header.width;
        let mut height = header.height;

        for i in 0..16 {
            if width == 0 || height == 0 {
                break;
            }

            let offset = header.mipmap_offsets[i] as usize;
            let length = header.mipmap_lengths[i] as usize;

            if length != 0 && offset + length <= buf.len() {
                let jpeg_chunk = &buf[offset..offset + length];
                let mut full_jpeg = Vec::with_capacity(jpeg_header_size + length);
                full_jpeg.extend_from_slice(&jpeg_header_chunk);
                full_jpeg.extend_from_slice(jpeg_chunk);

                let mut decoder = JpegDecoder::new(Cursor::new(&full_jpeg));

                match decoder.read_info() {
                    Ok(()) => {
                        let metadata = match decoder.info() {
                            Some(info) => info,
                            None => {
                                mipmaps.push(Mipmap { width, height, image: None });
                                width /= 2;
                                height /= 2;
                                continue;
                            }
                        };

                        let dec_w = metadata.width as u32;
                        let dec_h = metadata.height as u32;
                        let pixels = match decoder.decode() {
                            Ok(p) => p,
                            Err(_) => {
                                mipmaps.push(Mipmap { width, height, image: None });
                                width /= 2;
                                height /= 2;
                                continue;
                            }
                        };

                        if pixels.len() != (dec_w * dec_h * 4) as usize {
                            mipmaps.push(Mipmap { width, height, image: None });
                        } else {
                            let mut image = RgbaImage::new(width, height);
                            for (i, pixel) in image.pixels_mut().enumerate() {
                                let idx = i * 4;
                                *pixel = Rgba([
                                    255u8.saturating_sub(pixels[idx + 2]), // R
                                    255u8.saturating_sub(pixels[idx + 1]), // G
                                    255u8.saturating_sub(pixels[idx + 0]), // B
                                    255u8.saturating_sub(pixels[idx + 3]), // A
                                ]);
                            }

                            mipmaps.push(Mipmap { width, height, image: Some(image) });
                        }
                    }
                    Err(_) => {
                        mipmaps.push(Mipmap { width, height, image: None });
                    }
                }
            } else {
                mipmaps.push(Mipmap { width, height, image: None });
            };

            width /= 2;
            height /= 2;
        }

        Ok(Self { header, mipmaps })
    }
}
