use crate::header::Header;
use crate::texture_type::TextureType;
use byteorder::{LittleEndian, ReadBytesExt};
use image::{ImageFormat, RgbaImage};
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

    let mut max_covered = cursor.position() as usize;
    let mut decoded_levels = 0;

    for i in 0..16 {
        let offset = header.mipmap_offsets[i] as usize;
        let length = header.mipmap_lengths[i] as usize;
        if length == 0 {
            continue;
        }

        let end = offset + length;
        if end > buf.len() {
            println!("Skipping mipmap[{}]: out of bounds", i);
            continue;
        }

        max_covered = max_covered.max(end);
        decoded_levels += 1;
        println!("Mipmap[{}]: offset = {}, length = {}", i, offset, length);
    }

    println!("Total decoded mipmaps: {}", decoded_levels);
    println!("Max covered offset = {}", max_covered);
    println!("Buffer length = {}", buf.len());

    if max_covered == buf.len() {
        println!("✅ All mipmaps + header cover the file exactly.");
    } else if max_covered > buf.len() {
        println!("❌ Mipmaps exceed file length!");
    } else {
        println!("⚠️ File has trailing data: {} bytes", buf.len() - max_covered);
    }

    let offset = header.mipmap_offsets[0] as usize;
    let length = header.mipmap_lengths[0] as usize;

    if length == 0 || offset + length > buf.len() {
        return Err("No valid mipmap[0] found".into());
    }

    let jpeg_chunk = &buf[offset..offset + length];
    let mut full_jpeg = Vec::with_capacity(jpeg_header_size + length);
    full_jpeg.extend_from_slice(&jpeg_header_chunk);
    full_jpeg.extend_from_slice(jpeg_chunk);

    // Decode base image
    let mut rgb = image::ImageReader::with_format(Cursor::new(&full_jpeg), ImageFormat::Jpeg)
        .decode()
        .map_err(|e| format!("JPEG decode failed: {}", e))?
        .into_rgba8();

    // Decode alpha mask (BGRA)
    let mut decoder = JpegDecoder::new(Cursor::new(&full_jpeg));
    decoder.read_info()?;
    let metadata = decoder
        .info()
        .ok_or("Missing JPEG metadata")?;
    let pixels = decoder.decode()?;

    let expected = metadata.width as usize * metadata.height as usize * 4;
    if pixels.len() != expected {
        return Err("JPEG does not contain BGRA data (only RGB present)".into());
    }

    for (i, dst) in rgb.pixels_mut().enumerate() {
        let idx = i * 4;
        let alpha = pixels[idx + 3];

        *dst = image::Rgba([
            dst.0[2], // swap B↔R
            dst.0[1],
            dst.0[0],
            dst.0[3].saturating_sub(alpha),
        ]);
    }

    Ok(rgb)
}
