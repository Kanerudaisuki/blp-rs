use crate::err::error::BlpError;
use crate::image_blp::{ImageBlp, MAX_MIPS};

#[derive(Clone, Debug)]
pub struct EncoderOptions<'a> {
    /// JPEG quality (0..=100)
    pub quality: u8,
    /// Видимость мипов; если длина < MAX_MIPS — недостающие считаем true
    pub mip_visible: &'a [bool],
}
impl<'a> Default for EncoderOptions<'a> {
    fn default() -> Self {
        Self { quality: 85, mip_visible: &[] }
    }
}

/// Технический контекст, рассчитанный один раз на входе encode_blp
#[derive(Clone, Debug)]
pub struct EncoderCtx {
    pub quality: u8,
    pub base_width: u32,
    pub base_height: u32,
    pub has_alpha: bool,

    /// Эффективная маска видимости (ровно MAX_MIPS)
    pub eff_visible: [bool; MAX_MIPS],
    /// Индекс первого видимого мипа
    pub first_visible_mip: usize,
    /// Кол-во видимых среди реально присутствующих
    pub visible_count: usize,
    /// Сколько реально присутствует мипов в исходной картинке
    pub present_count: usize,
}

impl EncoderOptions<'_> {
    pub fn make_ctx(&self, img: &ImageBlp) -> Result<EncoderCtx, BlpError> {
        // база — мип0 должен быть
        let base = img
            .mipmaps
            .get(0)
            .and_then(|m| m.image.as_ref())
            .ok_or_else(|| BlpError::new("encode-blp.mip0-missing"))?;
        let (bw, bh) = (base.width(), base.height());
        if bw == 0 || bh == 0 {
            return Err(BlpError::new("encode-blp.empty-dim"));
        }
        let has_alpha = base.pixels().any(|p| p.0[3] != 255);

        // присутствие
        let present_count = img
            .mipmaps
            .iter()
            .take(MAX_MIPS)
            .filter(|m| m.image.is_some())
            .count()
            .max(1); // хотя бы мип0

        // эффективная видимость
        let mut eff = [true; MAX_MIPS];
        for i in 0..MAX_MIPS {
            if i < self.mip_visible.len() {
                eff[i] = self.mip_visible[i];
            }
        }

        // первый видимый из числа присутствующих
        let first_visible = (0..present_count)
            .find(|&i| eff[i])
            .ok_or_else(|| BlpError::new("encode-blp.no-visible"))?;

        // сколько реально видимых среди присутствующих
        let visible_count = (first_visible..present_count)
            .filter(|&i| eff[i])
            .count();

        Ok(EncoderCtx { quality: self.quality, base_width: bw, base_height: bh, has_alpha, eff_visible: eff, first_visible_mip: first_visible, visible_count, present_count })
    }
}
