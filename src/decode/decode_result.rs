use crate::image_blp::ImageBlp;

/// Сообщение из фонового декодера: единый путь — всегда ImageBlp или ошибка
pub enum DecodeResult {
    Blp(ImageBlp),
    Err(String),
}
