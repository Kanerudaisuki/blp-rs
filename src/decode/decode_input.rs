use crate::decode::decode_result::DecodeResult;
use crate::ui::viewer::file_picker::draw::decode_bytes;
use std::path::PathBuf;

pub enum DecodeInput {
    Path(PathBuf),
    Bytes(Vec<u8>),
}

pub fn decode_input(input: DecodeInput) -> DecodeResult {
    match input {
        DecodeInput::Path(path) => match std::fs::read(&path) {
            Ok(data) => decode_bytes(data),
            Err(e) => DecodeResult::Err(format!("Read failed: {e}")),
        },
        DecodeInput::Bytes(data) => decode_bytes(data),
    }
}
