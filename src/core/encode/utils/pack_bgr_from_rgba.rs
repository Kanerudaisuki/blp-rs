use image::RgbaImage;

pub fn pack_bgr_from_rgba(img: &RgbaImage) -> (Vec<u8>, usize) {
    let (w, h) = (img.width() as usize, img.height() as usize);
    let src = img.as_raw();
    let mut out = vec![0u8; w * h * 3];
    for (dst, px) in out
        .chunks_exact_mut(3)
        .zip(src.chunks_exact(4))
    {
        dst[0] = px[0]; // B
        dst[1] = px[1]; // G
        dst[2] = px[2]; // R
    }
    (out, w * 3)
}
