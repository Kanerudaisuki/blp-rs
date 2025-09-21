use crate::encode::blp::ctx::ctx::EncoderCtx;
use crate::encode::blp::options::EncoderOptions;
use crate::encode::blp::unit::unit::{MipSkipReason, MipUnit};
use crate::err::error::BlpError;
use crate::image_blp::{ImageBlp, MAX_MIPS};

impl ImageBlp {
    /// Build shifted layout (`mips`) and container-level context.
    ///
    /// - Computes presence & effective visibility.
    /// - Finds the first **present+visible** source mip and shifts the
    ///   destination so that `dst 0 → src first`.
    /// - Fills `mips` with exactly `MAX_MIPS` units; holes are allowed.
    /// - Detects alpha on the shifted base.
    pub fn build_layout_ctx(&self, opts: EncoderOptions) -> Result<EncoderCtx, BlpError> {
        // Presence (holes allowed).
        let mut present_mask = [false; MAX_MIPS];
        for (i, m) in self
            .mipmaps
            .iter()
            .take(MAX_MIPS)
            .enumerate()
        {
            present_mask[i] = m.image.is_some();
        }
        let present_count = present_mask
            .iter()
            .filter(|&&p| p)
            .count();

        // Effective visibility: default true, override with provided prefix.
        let mut eff_visible = [true; MAX_MIPS];
        for (dst, &v) in eff_visible
            .iter_mut()
            .zip(opts.mip_visible.iter().take(MAX_MIPS))
        {
            *dst = v;
        }

        // First index that is present AND visible.
        let first_src = (0..MAX_MIPS)
            .find(|&i| present_mask[i] && eff_visible[i])
            .ok_or_else(|| BlpError::new("encode_blp_no_visible"))?;

        // Base image/dimensions from that first source mip.
        let base_img = self.mipmaps[first_src]
            .image
            .as_ref()
            .ok_or_else(|| BlpError::new("encode_blp_mip_missing"))?;
        let (bw, bh) = (base_img.width(), base_img.height());
        if bw == 0 || bh == 0 {
            return Err(BlpError::new("encode_blp_empty_dim"));
        }
        let has_alpha = base_img.pixels().any(|p| p.0[3] != 255);

        // Build shifted `mips`: dst i → src (first_src + i)
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
                if eff_visible[src] {
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

        // Visible count = number of included units.
        let visible_count = units
            .iter()
            .filter(|u| u.included)
            .count();

        Ok(EncoderCtx {
            quality: opts.quality, //
            base_width: bw,
            base_height: bh,
            has_alpha,
            eff_visible,
            present_count,
            visible_count,
            bytes: Vec::new(),
            common_header_len: 0,
            total_slices_bytes: 0,
            mips: units,
            jpeg_plan: None,
            common_header: Vec::new(),
            alpha_depth: 0,
        })
    }
}
