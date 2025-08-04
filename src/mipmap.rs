use image::RgbaImage;

#[derive(Debug)]
pub struct Mipmap {
    pub width: u32,
    pub height: u32,
    pub image: Option<RgbaImage>,
}
