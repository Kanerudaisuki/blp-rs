// src/encode/blp/unit/unit.rs
use crate::encode::blp::jpeg::types::JpegSlices;

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum MipSkipReason {
    UserDisabled,
    NotPresent,
}

/// One encoded mip + bookkeeping.
#[derive(Clone, Debug)]
pub struct MipUnit {
    pub index: usize,             // dst index after shift
    pub src_index: Option<usize>, // source index in ImageBlp::mipmaps
    pub width: u32,
    pub height: u32,
    pub included: bool,

    /// Full JPEG bytes (SOI..EOI) — мы их кодируем mozjpeg'ом.
    pub jpeg_full: Vec<u8>,
    pub jpeg_full_bytes: usize,

    /// Срезы: где заканчивается head (включая SOS) и сколько длится scan.
    /// Нужны, чтобы в контейнер писать только scan.
    pub jpeg_slices: Option<JpegSlices>,

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
            jpeg_slices: None,
            encode_ms_acc: 0.0,
            skip_reason: Some(MipSkipReason::NotPresent),
        }
    }
}
