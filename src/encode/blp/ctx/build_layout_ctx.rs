// src/encode/blp/ctx/layout.rs (или рядом)

use crate::encode::blp::ctx::ctx::EncoderCtx;
use crate::encode::blp::options::EncoderOptions;
use crate::encode::blp::unit::unit::{MipSkipReason, MipUnit};
use crate::err::error::BlpError;
use crate::image_blp::{ImageBlp, MAX_MIPS};

impl ImageBlp {
    /// Собирает контекст: эффективная видимость, смещение мипов, `MipUnit`’ы,
    /// базовые размеры/альфа и счётчики. JPEG ещё не закодирован.
    pub fn build_layout_ctx(&self, opts: EncoderOptions) -> Result<EncoderCtx, BlpError> {
        // 1) Эффективная маска видимости
        let mut eff = [true; MAX_MIPS];
        for (d, &v) in eff
            .iter_mut()
            .zip(opts.mip_visible.iter().take(MAX_MIPS))
        {
            *d = v;
        }

        // 2) Маска присутствия
        let mut present_mask = [false; MAX_MIPS];
        let mut present_count = 0usize;
        for (i, m) in self
            .mipmaps
            .iter()
            .take(MAX_MIPS)
            .enumerate()
        {
            let present = m.image.is_some();
            present_mask[i] = present;
            if present {
                present_count += 1;
            }
        }

        // 3) Первый source, который present && visible
        let first_src = (0..MAX_MIPS)
            .find(|&i| present_mask[i] && eff[i])
            .ok_or_else(|| BlpError::new("encode_blp_no_visible"))?;

        // 4) Построение юнитов со смещением
        let mut units: Vec<MipUnit> = (0..MAX_MIPS)
            .map(MipUnit::empty)
            .collect();
        for dst in 0..MAX_MIPS {
            let src = first_src + dst;
            let u = &mut units[dst];
            u.index = dst;
            if src < MAX_MIPS && present_mask[src] {
                u.src_index = Some(src);
                let mip = &self.mipmaps[src];
                u.width = mip.width;
                u.height = mip.height;

                if eff[src] {
                    u.included = true;
                    u.skip_reason = None;
                } else {
                    u.included = false;
                    u.skip_reason = Some(MipSkipReason::UserDisabled);
                }
            } else {
                u.src_index = None;
                u.included = false;
                u.skip_reason = Some(MipSkipReason::NotPresent);
            }
        }

        // 5) База и альфа — по первому включённому dst-юниту
        let base_idx = units
            .iter()
            .position(|u| u.included)
            .ok_or_else(|| BlpError::new("encode_blp_no_visible"))?;
        let (bw, bh) = (units[base_idx].width, units[base_idx].height);
        if bw == 0 || bh == 0 {
            return Err(BlpError::new("encode_blp_empty_dim"));
        }
        let has_alpha = units[base_idx]
            .src_index
            .and_then(|src| self.mipmaps[src].image.as_ref())
            .map(|im| im.pixels().any(|p| p.0[3] != 255))
            .unwrap_or(false);

        // 6) Счётчики
        let visible_count = units
            .iter()
            .filter(|u| u.included)
            .count();

        Ok(EncoderCtx { quality: opts.quality, base_width: bw, base_height: bh, has_alpha, eff_visible: eff, present_count, visible_count, bytes: Vec::new(), common_header_len: 0, total_slices_bytes: 0, mips: units })
    }
}
