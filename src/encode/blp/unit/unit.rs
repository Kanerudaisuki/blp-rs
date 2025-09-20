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
    pub index: usize,            // dst-индекс после смещения (0..MAX_MIPS-1)
    pub src_index: Option<usize>,// исходный индекс в ImageBlp::mipmaps (None = not present)
    pub width: u32,
    pub height: u32,
    pub included: bool,
    pub jpeg_full: Vec<u8>,
    pub jpeg_full_bytes: usize,
    pub encode_ms_acc: f64,
    pub skip_reason: Option<MipSkipReason>,
}

impl MipUnit {
    pub fn empty(index: usize) -> Self {
        Self {
            index, //
            src_index: None,
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
