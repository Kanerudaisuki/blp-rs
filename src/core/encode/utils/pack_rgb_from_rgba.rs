#[inline]
#[allow(dead_code)]
pub fn pack_rgb_from_rgba(img: &image::RgbaImage) -> (Vec<u8>, usize) {
    let (w, h) = img.dimensions();
    let src = img.as_raw();
    let mut out = vec![0u8; (w as usize) * (h as usize) * 3];
    for (dst, px) in out
        .chunks_exact_mut(3)
        .zip(src.chunks_exact(4))
    {
        dst[0] = px[0]; // R
        dst[1] = px[1]; // G
        dst[2] = px[2]; // B
    }
    (out, w as usize * 3)
}
