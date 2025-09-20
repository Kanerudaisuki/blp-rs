use crate::encode::blp::options::EncoderOptions;
use crate::encode::blp::unit::unit::MipUnit;
use crate::err::error::BlpError;
use crate::image_blp::{ImageBlp, MAX_MIPS};

/// Encoder context calculated once per encoding run.
///
/// Holds container-level parameters and output buffers. All per-mip details
/// live in `MipUnit`.
///
/// Notes:
/// - Visibility “holes” are allowed (e.g. `[false, true, false, …]`).
/// - The mip layout used for encoding is *shifted* so that destination mip 0
///   corresponds to the first source mip that is both present and visible.
/// - `base_width`/`base_height` refer to that shifted base level.
///
#[derive(Clone, Debug)]
pub struct EncoderCtx {
    /// JPEG quality in the range `0..=100`.
    pub quality: u8,

    /// Width (in pixels) of the container’s base level **after the shift**.
    pub base_width: u32,

    /// Height (in pixels) of the container’s base level **after the shift**.
    pub base_height: u32,

    /// Whether the chosen base level contains any non-opaque pixels (`A != 255`).
    ///
    /// This flag influences the JPEG pixel format:
    /// - `true`  → encode as CMYK with `K = A` (4 BPP),
    /// - `false` → encode as BGR (3 BPP).
    pub has_alpha: bool,

    /// Effective visibility mask for all `MAX_MIPS` levels.
    ///
    /// If `EncoderOptions::mip_visible` is shorter than `MAX_MIPS`,
    /// the missing entries are treated as `true`.
    pub eff_visible: [bool; MAX_MIPS],

    /// Total number of source mips that are present in the input (holes allowed).
    pub present_count: usize,

    /// Total number of mips that are both present **and** visible (holes allowed).
    pub visible_count: usize,

    // -------- Output container (filled by the writer) --------
    /// Final BLP container bytes produced by the writer.
    ///
    /// After calling `EncoderCtx::write_blp_full_jpegs(&mips)`, this buffer
    /// contains a complete BLP1 file.
    pub bytes: Vec<u8>,

    /// Length of the common JPEG header in bytes.
    ///
    /// Current phase keeps full JPEGs per mip, so this is `0`.
    pub common_header_len: usize,

    /// Sum of all included mip payload sizes (in bytes).
    pub total_slices_bytes: usize,

    pub mips: Vec<MipUnit>,
}

impl<'a> TryFrom<(&'a ImageBlp, EncoderOptions<'a>)> for EncoderCtx {
    type Error = BlpError;

    fn try_from((img, opts): (&ImageBlp, EncoderOptions)) -> Result<Self, BlpError> {
        // Presence (holes allowed).
        let mut present_mask = [false; MAX_MIPS];
        let mut present_count = 0usize;
        for (i, m) in img
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

        // Effective visibility: default true, override with provided prefix.
        let mut eff_visible = [true; MAX_MIPS];
        for (dst, &v) in eff_visible
            .iter_mut()
            .zip(opts.mip_visible.iter().take(MAX_MIPS))
        {
            *dst = v;
        }

        // First index that is present AND visible.
        let first_visible_src = (0..MAX_MIPS)
            .find(|&i| present_mask[i] && eff_visible[i])
            .ok_or_else(|| BlpError::new("encode_blp_no_visible"))?;

        // Base dimensions taken from that first present+visible source mip.
        let base_img = img.mipmaps[first_visible_src]
            .image
            .as_ref()
            .ok_or_else(|| BlpError::new("encode_blp_mip_missing"))?;
        let (bw, bh) = (base_img.width(), base_img.height());
        if bw == 0 || bh == 0 {
            return Err(BlpError::new("encode_blp_empty_dim"));
        }

        // Alpha detection on that same mip.
        let has_alpha = base_img.pixels().any(|p| p.0[3] != 255);

        // Total count of present+visible across the whole range (holes allowed).
        let visible_count = (0..MAX_MIPS)
            .filter(|&i| present_mask[i] && eff_visible[i])
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
            mips: Vec::new(),
        })
    }
}
