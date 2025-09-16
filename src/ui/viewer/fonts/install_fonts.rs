use crate::ui::viewer::fonts::fonts::{FONT_BYTES, CJK_FONT_BYTES, CJK_TC_FONT_BYTES, FontFace};
use eframe::epaint::FontFamily;
use eframe::epaint::text::{FontData, FontDefinitions};
use std::sync::Arc;
use eframe::egui::Context;

/// Регистрирует все шрифты и делает Regular дефолтным
pub fn install_fonts(ctx: &Context) {
    let mut defs = FontDefinitions::default();

    // Регистрируем CJK шрифты
    for (face, bytes) in CJK_FONT_BYTES {
        defs.font_data
            .insert(face.cjk_key().into(), Arc::from(FontData::from_static(bytes)));
    }

    // Регистрируем CJK TC шрифты
    for (face, bytes) in CJK_TC_FONT_BYTES {
        defs.font_data
            .insert(face.cjk_tc_key().into(), Arc::from(FontData::from_static(bytes)));
    }

    // Регистрируем байты JetBrains Mono
    for (face, bytes) in FONT_BYTES {
        defs.font_data
            .insert(face.key().into(), Arc::from(FontData::from_static(bytes)));
    }

    // TC шрифт — первый для пропорционального семейства
    // CJK шрифт — второй для пропорционального семейства
    // Regular JetBrains Mono — дефолт для моноширинного семейства
    let cjk_tc_regular: String = FontFace::Regular.cjk_tc_key().into();
    let cjk_regular: String = FontFace::Regular.cjk_key().into();
    let regular: String = FontFace::Regular.key().into();
    defs.families
        .get_mut(&FontFamily::Proportional)
        .unwrap()
        .insert(0, cjk_tc_regular);
    defs.families
        .get_mut(&FontFamily::Proportional)
        .unwrap()
        .insert(1, cjk_regular);
    defs.families
        .get_mut(&FontFamily::Monospace)
        .unwrap()
        .insert(0, regular);

    ctx.set_fonts(defs);
}
