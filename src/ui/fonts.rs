use eframe::egui::{Context, FontFamily};
use eframe::epaint::text::{FontData, FontDefinitions};
use std::io::Read;
use std::sync::Arc;

// сгенерировано build.rs: [("StemName", &[u8 .br]), ...]
include!(concat!(env!("OUT_DIR"), "/fonts_gen.rs"));

#[inline]
fn br_to_ttf_vec(br: &[u8]) -> Vec<u8> {
    use brotli_decompressor::Decompressor;
    let mut out = Vec::new();
    let mut rd = Decompressor::new(br, 4096);
    rd.read_to_end(&mut out)
        .expect("brotli decompress");
    out
}

#[inline]
fn insert_owned(defs: &mut FontDefinitions, key: &str, br: &[u8]) {
    defs.font_data
        .insert(key.to_string(), Arc::from(FontData::from_owned(br_to_ttf_vec(br))));
}

fn push_front_unique(defs: &mut FontDefinitions, family: FontFamily, name: &str) {
    if !defs.font_data.contains_key(name) {
        return;
    }
    let v = defs
        .families
        .get_mut(&family)
        .expect("family");
    if let Some(pos) = v.iter().position(|s| s == name) {
        v.remove(pos);
    }
    v.insert(0, name.to_string());
}

#[inline]
fn has(defs: &FontDefinitions, name: &str) -> bool {
    defs.font_data.contains_key(name)
}

/// База: JetBrainsMono-Regular для обоих семейств.
/// Фоллбеки: CJK TC → CJK SC.
pub fn install_fonts(ctx: &Context) {
    let mut defs = FontDefinitions::default();

    // 1) зарегистрировать всё из OUT_DIR
    for (stem, br) in all_fonts_br() {
        insert_owned(&mut defs, stem, br);
    }

    // ключи (из твоей папки)
    let jb = "JetBrainsMono-Regular";
    let tc = "LXGWWenKaiMonoTC-Regular";
    let sc = "LXGWWenKaiMono-Regular";

    // 2) Proportional: JB → TC → SC

    if has(&defs, tc) {
        push_front_unique(&mut defs, FontFamily::Proportional, tc);
    }
    if has(&defs, sc) {
        push_front_unique(&mut defs, FontFamily::Proportional, sc);
    }
    if has(&defs, jb) {
        push_front_unique(&mut defs, FontFamily::Proportional, jb);
    }

    // 3) Monospace: JB → TC → SC
    if has(&defs, tc) {
        push_front_unique(&mut defs, FontFamily::Monospace, tc);
    }
    if has(&defs, sc) {
        push_front_unique(&mut defs, FontFamily::Monospace, sc);
    }
    if has(&defs, jb) {
        push_front_unique(&mut defs, FontFamily::Monospace, jb);
    }

    // 4) Применить
    ctx.set_fonts(defs);
}
