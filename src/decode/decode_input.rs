use crate::err::app_err::AppErr;
use crate::image_blp::ImageBlp;
use std::fs;
use std::path::PathBuf;

pub enum DecodeInput {
    Path(PathBuf),
    Bytes(Vec<u8>),
}

pub fn decode_input(input: DecodeInput) -> Result<ImageBlp, AppErr> {
    match input {
        DecodeInput::Path(path) => match fs::read(&path) {
            Ok(data) => decode_bytes(data),
            //Err(e) => Err(err_wire_cause!(ErrKind::OpenPath { path: path.display().to_string() }, &e)),
            Err(e) => Err(AppErr::new("test")),
        },
        DecodeInput::Bytes(data) => decode_bytes(data),
    }
}

fn decode_bytes(data: Vec<u8>) -> Result<ImageBlp, AppErr> {
    match ImageBlp::from_buf(&data) {
        Ok(blp) => Ok(blp),
        //Err(e) => Err(err_wire_cause!(ErrKind::DecodeImage { guessed: Some("blp"), size: data.len() }, e.as_ref())),
        Err(e) => Err(AppErr::new("test")),
    }
}
