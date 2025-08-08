use crate::ui::viewer::app::App;
use eframe::egui::{Context, FontData, FontDefinitions, FontFamily, FontId, Style, TextStyle};
use std::sync::Arc;

#[derive(Clone, Copy)]
pub enum FontName {
    Regular,
    Bold,
    Medium,
    Italic,
}

impl FontName {
    pub const fn as_str(self) -> &'static str {
        match self {
            FontName::Regular => "JBMono-Regular",
            FontName::Bold => "JBMono-Bold",
            FontName::Medium => "JBMono-Medium",
            FontName::Italic => "JBMono-Italic",
        }
    }
}

pub const FONT_FILES: &[(FontName, &[u8])] = &[
    (FontName::Regular, include_bytes!("ttf/JetBrainsMono-Regular.ttf")), //
    (FontName::Bold, include_bytes!("ttf/JetBrainsMono-Bold.ttf")),
    (FontName::Medium, include_bytes!("ttf/JetBrainsMono-Medium.ttf")),
    (FontName::Italic, include_bytes!("ttf/JetBrainsMono-Italic.ttf")),
];

#[derive(Clone, Copy)]
pub enum FontPreset {
    Title,
    Controls,
    Body,
    MonoSmall,
    Mono,
}

impl FontPreset {
    pub fn font_id(self) -> FontId {
        match self {
            FontPreset::Title => FontId::new(14.0, FontFamily::Name(FontName::Bold.as_str().into())),
            FontPreset::Controls => FontId::new(13.0, FontFamily::Name(FontName::Medium.as_str().into())),
            FontPreset::Body => FontId::new(14.0, FontFamily::Proportional),
            FontPreset::MonoSmall => FontId::new(12.0, FontFamily::Monospace),
            FontPreset::Mono => FontId::new(13.0, FontFamily::Monospace),
        }
    }
}

impl App {
    pub(crate) fn install_fonts_and_style(&self, ctx: &Context) {
        let mut fonts = FontDefinitions::default();

        // Регистрируем все шрифты из константы
        for (name, bytes) in FONT_FILES {
            fonts
                .font_data
                .insert(name.as_str().into(), Arc::from(FontData::from_static(bytes)));
        }

        // Делаем Regular дефолтным
        fonts
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .insert(0, FontName::Regular.as_str().into());
        fonts
            .families
            .get_mut(&FontFamily::Monospace)
            .unwrap()
            .insert(0, FontName::Regular.as_str().into());

        // Базовые размеры
        let mut style: Style = (*ctx.style()).clone();
        style
            .text_styles
            .insert(TextStyle::Body, FontId::new(14.0, FontFamily::Proportional));
        style
            .text_styles
            .insert(TextStyle::Button, FontId::new(13.0, FontFamily::Proportional));
        style
            .text_styles
            .insert(TextStyle::Heading, FontId::new(16.0, FontFamily::Proportional));

        ctx.set_fonts(fonts);
        ctx.set_style(style);
    }
}
