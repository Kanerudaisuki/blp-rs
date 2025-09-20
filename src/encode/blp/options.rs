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
