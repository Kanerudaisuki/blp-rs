use crate::encode::blp::unit::unit::MipUnit;
use crate::image_blp::MAX_MIPS;
use std::fmt;

#[derive(Clone, Debug)]
pub struct EncodeReport {
    pub bytes: Vec<u8>, // готовый BLP
    pub base_width: u32,
    pub base_height: u32,
    pub first_visible_mip: usize,
    pub visible_count: usize,
    pub has_alpha: bool,
    pub common_header_len: usize,  // сейчас 0 (кладём полные JPEG)
    pub total_slices_bytes: usize, // суммарно по всем включённым
    pub effective_mip_visible: [bool; MAX_MIPS],
    pub mips: Vec<MipUnit>, // РОВНО MAX_MIPS, служебные данные на выход
}

impl fmt::Display for EncodeReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "== BLP encode report ==")?;
        writeln!(f, "bytes: {} ({:.6} KiB)", self.bytes.len(), self.bytes.len() as f64 / 1024.0)?;
        writeln!(f, "container base: {}x{} (first visible mip = {})", self.base_width, self.base_height, self.first_visible_mip)?;
        writeln!(f, "has_alpha: {}", self.has_alpha)?;
        writeln!(f, "visible mips: {}", self.visible_count)?;
        writeln!(f, "common header length: {} bytes", self.common_header_len)?;
        for m in &self.mips {
            if m.included {
                writeln!(f, "mip{}: {}x{} ({} bytes, {:.2} KiB), encode: {:.3} ms", m.index, m.width, m.height, m.jpeg_full_bytes, m.jpeg_full_bytes as f64 / 1024.0, m.encode_ms_acc)?;
            } else {
                // причина уже заложена в MipUnit.skip_reason, но в отчёт не расписываем — коротко
                writeln!(f, "mip{}: SKIPPED", m.index)?;
            }
        }
        writeln!(f, "total slices bytes: {}", self.total_slices_bytes)?;
        Ok(())
    }
}
