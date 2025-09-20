/// Reason why a mip level was skipped during encoding.
#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum MipSkipReason {
    /// The user explicitly disabled this mip level (via options).
    UserDisabled,
    /// The mip level is not present (image too small or not generated).
    NotPresent,
}

/// Represents a single mip level unit during encoding.
///
/// Each `MipUnit` stores metadata and the full encoded JPEG data for one
/// mip level. This is a working structure used by the encoder to track
/// included/excluded mips, timing, and encoded byte size.
#[derive(Clone, Debug)]
pub struct MipUnit {
    /// Index of the mip level (0 = base).
    pub index: usize,
    /// Width of this mip level in pixels.
    pub width: u32,
    /// Height of this mip level in pixels.
    pub height: u32,
    /// Whether this mip level is included in the container.
    pub included: bool,
    /// The full JPEG data for this mip (SOI..EOI).
    pub jpeg_full: Vec<u8>,
    /// Cached length of `jpeg_full` (for quick checks).
    pub jpeg_full_bytes: usize,
    /// Accumulated encoding time in milliseconds for this mip.
    pub encode_ms_acc: f64,
    /// Reason why this mip level was skipped, if not included.
    pub skip_reason: Option<MipSkipReason>,
}

impl MipUnit {
    /// Creates an "empty" placeholder mip unit.
    ///
    /// Used to fill the fixed-length mip array up to `MAX_MIPS`.
    /// The unit is marked as not included and has a skip reason `NotPresent`.
    pub fn empty(index: usize) -> Self {
        Self {
            index, //
            width: 0,
            height: 0,
            included: false,
            jpeg_full: Vec::new(),
            jpeg_full_bytes: 0,
            encode_ms_acc: 0.0,
            skip_reason: Some(MipSkipReason::NotPresent),
        }
    }
}
