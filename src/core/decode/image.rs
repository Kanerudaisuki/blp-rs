use crate::core::image::{ImageBlp, MAX_MIPS};
use crate::error::error::BlpError;
use image::{
    self,
    imageops::{FilterType, crop_imm, resize},
};

impl ImageBlp {
    /// Внешнее изображение: scale-to-cover + центр-кроп до (self.width,self.height), затем генерация мипов.
    pub(crate) fn decode_image(&mut self, buf: &[u8]) -> Result<(), BlpError> {
        let src = image::load_from_memory(buf)?.to_rgba8();

        let (tw, th) = (self.width.max(1), self.height.max(1));
        let (sw, sh) = src.dimensions();

        // 1) cover-скейл без искажения аспектов
        let sx = tw as f32 / sw as f32;
        let sy = th as f32 / sh as f32;
        let s = sx.max(sy);
        let rw = (sw as f32 * s).ceil() as u32;
        let rh = (sh as f32 * s).ceil() as u32;
        let resized = resize(&src, rw, rh, FilterType::Triangle);

        // 2) центр-кроп в ровно (tw,th)
        let cx = ((rw - tw) / 2).min(rw.saturating_sub(tw));
        let cy = ((rh - th) / 2).min(rh.saturating_sub(th));
        let base = crop_imm(&resized, cx, cy, tw, th).to_image();

        // 3) цепочка мипов
        let mut prev = base;
        let (mut w, mut h) = (tw, th);
        for i in 0..MAX_MIPS {
            if i >= self.mipmaps.len() {
                break;
            }
            self.mipmaps[i].image = Some(prev.clone());
            if w == 1 && h == 1 {
                break;
            }
            w = (w / 2).max(1);
            h = (h / 2).max(1);
            prev = resize(&prev, w, h, FilterType::Lanczos3);
        }
        Ok(())
    }
}
