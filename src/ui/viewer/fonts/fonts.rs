use egui::{Context, FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

/// Названия шрифтовых гарнитур
#[derive(Clone, Copy)]
pub enum FontFace {
    Regular,
    Medium,
    Bold,
    Italic,
}

impl FontFace {
    pub const fn key(self) -> &'static str {
        match self {
            FontFace::Regular => "JBMono-Regular",
            FontFace::Medium => "JBMono-Medium",
            FontFace::Bold => "JBMono-Bold",
            FontFace::Italic => "JBMono-Italic",
        }
    }
}

/// Вшитые TTF-байты
pub const FONT_BYTES: &[(FontFace, &[u8])] = &[
    (FontFace::Regular, include_bytes!("ttf/JetBrainsMono-Regular.ttf")), //
    (FontFace::Medium, include_bytes!("ttf/JetBrainsMono-Medium.ttf")),
    (FontFace::Bold, include_bytes!("ttf/JetBrainsMono-Bold.ttf")),
    (FontFace::Italic, include_bytes!("ttf/JetBrainsMono-Italic.ttf")),
];

