//! Encoder context calculated once per encoding run.
//!
//! Holds container-level parameters and output buffers. All per-mip details
//! live in `MipUnit`.
//!
//! Notes:
//! - Visibility “holes” are allowed (e.g. `[false, true, false, …]`).
//! - The mip layout used for encoding is *shifted* so that destination mip 0
//!   corresponds to the first source mip that is both present and visible.
//! - `base_width`/`base_height` refer to that shifted base level.

use crate::encode::blp::jpeg::types::JpegPlan;
use crate::encode::blp::unit::unit::MipUnit;
use crate::image_blp::MAX_MIPS;

/// Encoder context calculated once per encoding run.
///
/// Keeps container-level knobs/flags and the output buffer. Per-mip data is
/// stored in `mips`.
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
    /// After calling `EncoderCtx::write_blp_full_jpegs()`, this buffer
    /// contains a complete BLP1 file.
    pub bytes: Vec<u8>,

    /// Length of the common JPEG header in bytes.
    ///
    /// Current phase keeps full JPEGs per mip, so this is `0`.
    pub common_header_len: usize,

    /// Sum of all included mip payload sizes (in bytes).
    pub total_slices_bytes: usize,

    /// Shifted destination layout (always `MAX_MIPS` entries).
    pub mips: Vec<MipUnit>,

    /// Reference JPEG plan captured from the first *included* mip.
    /// Used to validate that all mips share identical DQT/DHT/DRI/SOF0/SOS.
    pub jpeg_plan: Option<JpegPlan>,

    /// Общий заголовок: SOI + DQT* + DHT* + DRI? (без SOF0/SOS).
    pub common_header: Vec<u8>,

    /// Alpha depth for BLP1 header: 0 or 8 (derived from `has_alpha`).
    pub alpha_depth: u8,
}
