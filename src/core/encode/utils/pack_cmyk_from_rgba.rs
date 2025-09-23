use image::RgbaImage;

pub fn pack_cmyk_from_rgba(img: &RgbaImage) -> (Vec<u8>, usize) {
    let (w, h) = (img.width() as usize, img.height() as usize);
    let src = img.as_raw();
    let mut out = vec![0u8; w * h * 4];
    for (dst, px) in out
        .chunks_exact_mut(4)
        .zip(src.chunks_exact(4))
    {
        dst[0] = 255u8.saturating_sub(px[0]); // C = 255-R
        dst[1] = 255u8.saturating_sub(px[1]); // M = 255-G
        dst[2] = 255u8.saturating_sub(px[2]); // Y = 255-B
        dst[3] = 255u8.saturating_sub(px[3]); // K = 255-A
    }
    (out, w * 4)
}
