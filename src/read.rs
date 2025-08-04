use crate::header::Header;
use crate::texture_type::TextureType;
use byteorder::{LittleEndian, ReadBytesExt};
use image::RgbaImage;
use jpeg_decoder::Decoder as JpegDecoder;
use std::error::Error;
use std::io::{Cursor, Read};

pub fn convert_blp_to_rgba_image(buf: &[u8]) -> Result<RgbaImage, Box<dyn Error + Send + Sync>> {
    let mut cursor = Cursor::new(buf);
    let header = Header::parse(&mut cursor)?;
    println!("{:#?}", header);

    if header.texture_type != TextureType::JPEG {
        return Err("Only JPEG-encoded BLP is supported in this function".into());
    }

    if cursor.position() != 156 {
        println!("Warning: unexpected cursor position, got {}", cursor.position());
    }

    let jpeg_header_size = cursor.read_u32::<LittleEndian>()? as usize;
    let mut jpeg_header_chunk = vec![0u8; jpeg_header_size];
    cursor.read_exact(&mut jpeg_header_chunk)?;

    let offset = header.mipmap_offsets[0] as usize;
    let length = header.mipmap_lengths[0] as usize;
    if length == 0 || offset + length > buf.len() {
        return Err("No valid mipmap[0] found".into());
    }

    let jpeg_chunk = &buf[offset..offset + length];
    let mut full_jpeg = Vec::with_capacity(jpeg_header_size + length);
    full_jpeg.extend_from_slice(&jpeg_header_chunk);
    full_jpeg.extend_from_slice(jpeg_chunk);

    let mut decoder = JpegDecoder::new(Cursor::new(&full_jpeg));
    decoder.read_info()?;
    let metadata = decoder
        .info()
        .ok_or("Missing JPEG metadata")?;

    let width = metadata.width as u32;
    let height = metadata.height as u32;
    let pixels = decoder.decode()?;

    let expected = (width * height * 4) as usize;
    if pixels.len() != expected {
        return Err("JPEG does not contain BGRA data (only RGB present)".into());
    }

    let mut image = RgbaImage::new(width, height);
    for (i, pixel) in image.pixels_mut().enumerate() {
        let idx = i * 4;
        *pixel = image::Rgba([
            255u8.saturating_sub(pixels[idx + 2]), //
            255u8.saturating_sub(pixels[idx + 1]),
            255u8.saturating_sub(pixels[idx + 0]),
            255u8.saturating_sub(pixels[idx + 3]),
        ]);
    }

    Ok(image)
}
