use crate::util::floor_pow2::floor_pow2;
use image::RgbaImage;
use image::imageops::crop_imm;

pub fn center_crop_to_pow2(src: &RgbaImage) -> RgbaImage {
    let (w0, h0) = src.dimensions();
    let w1 = floor_pow2(w0);
    let h1 = floor_pow2(h0);
    let x0 = (w0 - w1) / 2;
    let y0 = (h0 - h1) / 2;
    crop_imm(src, x0, y0, w1, h1).to_image()
}
