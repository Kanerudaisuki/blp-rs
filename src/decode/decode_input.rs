use crate::decode::decode_result::DecodeResult;
use crate::image_blp::ImageBlp;
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
fn decode_bytes(data: Vec<u8>) -> DecodeResult {
    match ImageBlp::from_buf(&data) {
        Ok(blp) => {
            if blp.mipmaps.is_empty() {
                DecodeResult::Err("empty BLP mip chain".into())
            } else {
                DecodeResult::Blp(blp)
            }
        }
        Err(e) => DecodeResult::Err(format!("from_bytes failed: {e}")),
    }
}
