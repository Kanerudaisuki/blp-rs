use crate::header::Header;
use image::RgbaImage;
use std::error::Error;
use std::io::Cursor;

pub fn convert_blp_to_rgba_image(buf: &[u8]) -> Result<RgbaImage, Box<dyn Error + Send + Sync>> {
    let mut cursor = Cursor::new(buf);
    let header = Header::parse(&mut cursor)?;
    println!("{:#?}", header);

    Err("BLP decoding not yet implemented".into())
}
