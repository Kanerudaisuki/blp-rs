use image::RgbaImage;
use std::error::Error;
use std::io::Cursor;

pub fn jpeg_get_alpha(data: &[u8]) -> Result<RgbaImage, Box<dyn Error + Send + Sync>> {
    let mut decoder = jpeg_decoder::Decoder::new(Cursor::new(data));
    decoder.read_info()?;

    let metadata = decoder
        .info()
        .ok_or("Missing JPEG metadata")?;
    let mut pixels = decoder.decode()?;

    let expected = metadata.width as usize * metadata.height as usize * 4;
    if pixels.len() == expected {
        // BGRA → RGBA
        let mut rgba = RgbaImage::new(metadata.width.into(), metadata.height.into());
        for (i, pixel) in rgba.pixels_mut().enumerate() {
            let idx = i * 4;
            *pixel = image::Rgba([
                pixels[idx + 2], // R
                pixels[idx + 1], // G
                pixels[idx + 0], // B
                pixels[idx + 3], // A
            ]);
        }
        Ok(rgba)
    } else {
        Err("JPEG does not contain BGRA data (only RGB present)".into())
    }
}
