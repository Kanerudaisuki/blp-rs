use crate::ui::viewer::fonts::fonts::{FONT_BYTES, FontFace};
use eframe::epaint::FontFamily;
use eframe::epaint::text::{FontData, FontDefinitions};
use egui::Context;
use std::sync::Arc;

/// Регистрирует все шрифты и делает Regular дефолтным
pub fn install_fonts(ctx: &Context) {
    let mut defs = FontDefinitions::default();

    // Регистрируем байты
    for (face, bytes) in FONT_BYTES {
        defs.font_data
            .insert(face.key().into(), Arc::from(FontData::from_static(bytes)));
    }

    // Regular — дефолт для пропорционального и моноширинного семейств
    let regular: String = FontFace::Regular.key().into();
    defs.families
        .get_mut(&FontFamily::Proportional)
        .unwrap()
        .insert(0, regular.clone());
    defs.families
        .get_mut(&FontFamily::Monospace)
        .unwrap()
        .insert(0, regular);

    ctx.set_fonts(defs);
}
