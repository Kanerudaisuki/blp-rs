use crate::header::Header;
use crate::texture_type::TextureType;
use byteorder::{LittleEndian, ReadBytesExt};
use image::{ImageFormat, RgbaImage};
use std::error::Error;
use std::io::{Cursor, Read};

pub fn convert_blp_to_rgba_image(buf: &[u8]) -> Result<RgbaImage, Box<dyn Error + Send + Sync>> {
    let mut cursor = Cursor::new(buf);
    let header = Header::parse(&mut cursor)?;
    println!("{:#?}", header);

    // sanity-check: JPEG only
    if header.texture_type != TextureType::JPEG {
        return Err("Only JPEG-encoded BLP is supported in this function".into());
    }

    // Чтение jpegHeaderChunk из курсора
    let jpeg_header_size = cursor.read_u32::<LittleEndian>()? as usize;
    let mut jpeg_header_chunk = vec![0u8; jpeg_header_size];
    cursor.read_exact(&mut jpeg_header_chunk)?;

    // Поиск первого валидного mipmap
    for i in 0..16 {
        let offset = header.mipmap_offsets[i] as usize;
        let length = header.mipmap_lengths[i] as usize;

        if length == 0 || offset + length > buf.len() {
            continue;
        }

        let jpeg_chunk = &buf[offset..offset + length];
        let mut full_jpeg = Vec::with_capacity(jpeg_header_size + length);
        full_jpeg.extend_from_slice(&jpeg_header_chunk);
        full_jpeg.extend_from_slice(jpeg_chunk);

        // Декодирование JPEG
        let dyn_img = image::ImageReader::with_format(Cursor::new(full_jpeg), ImageFormat::Jpeg)
            .decode()
            .map_err(|e| format!("JPEG decode failed at mipmap[{}]: {}", i, e))?;

        let mut rgba = dyn_img.into_rgba8();

        // Перестановка B и R (BGRA → RGBA)
        for pixel in rgba.pixels_mut() {
            let r = pixel[0];
            pixel[0] = pixel[2];
            pixel[2] = r;
        }

        return Ok(rgba);
    }

    let remaining = buf
        .len()
        .saturating_sub(cursor.position() as usize);
    Err(format!("No valid mipmap found. Remaining bytes: {}", remaining).into())
}
