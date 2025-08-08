use crate::ui::viewer::app::App;
use crate::ui::viewer::fonts::fonts::{FONT_FILES, FontName};
use eframe::egui::{Context, FontData, FontDefinitions, FontFamily};
use egui::{FontId, Style, TextStyle};
use std::sync::Arc;
// enum FontName с as_str()

impl App {
    /// Глобальный стиль + регистрация всех шрифтов.
    /// НИКАКИХ строковых литералов внутри метода.
    pub(crate) fn apply_global_style(&self, ctx: &Context) {
        let mut fonts = FontDefinitions::default();

        // Регистрируем всё из константы
        for (name, bytes) in FONT_FILES {
            fonts
                .font_data
                .insert((*name).as_str().into(), Arc::from(FontData::from_static(bytes)));
        }

        // Делает Regular дефолтным семейством
        let regular: String = FontName::Regular.as_str().into();
        fonts
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .insert(0, regular.clone());
        fonts
            .families
            .get_mut(&FontFamily::Monospace)
            .unwrap()
            .insert(0, regular);

        // Базовые размеры для стандартных стилей (по желанию)
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
