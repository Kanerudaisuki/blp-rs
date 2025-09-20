use crate::encode::blp::unit::unit::{MipSkipReason, MipUnit};
use crate::image_blp::{ImageBlp, MAX_MIPS};

impl ImageBlp {
    /// Build a shifted list of `MipUnit`s so that destination mip 0 corresponds
    /// to the **first source mip that is present AND visible**. Holes are allowed
    /// afterwards (e.g., [false, true, false, true] is valid).
    ///
    /// - `eff_visible` is the effective visibility mask of length `MAX_MIPS`.
    /// - Returns a vector of exactly `MAX_MIPS` units and the total number of
    ///   present source mips (`present_count`) across the whole range.
    pub fn build_units_with_shift(&self, eff_visible: &[bool; MAX_MIPS]) -> (Vec<MipUnit>, usize) {
        let mut units: Vec<MipUnit> = (0..MAX_MIPS)
            .map(MipUnit::empty)
            .collect();

        // Presence mask + total present count (holes allowed).
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

        // First source index that is present AND visible.
        // Если такого нет (теоретически), сдвига не будет — возьмём первый present.
        let first_src = (0..MAX_MIPS)
            .find(|&i| present_mask[i] && eff_visible[i])
            .or_else(|| (0..MAX_MIPS).find(|&i| present_mask[i]))
            .unwrap_or(0);

        // Shift: dst 0 -> src first_src, dst 1 -> src first_src + 1, ...
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

        (units, present_count)
    }
}
