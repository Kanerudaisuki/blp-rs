use crate::ui::i18n::lng_list::LngList;
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
pub struct LngPrefs {
    pub lang: LngList,
    pub initialized: bool,
}

const APP: &str = env!("CARGO_PKG_NAME");
const CFG: Option<&str> = Some(stringify!(LngPrefs));

fn detect_os_lang() -> LngList {
    let raw = sys_locale::get_locale()
        .unwrap_or_default()
        .to_lowercase();
    match raw
        .split(|c| c == '-' || c == '_')
        .next()
        .unwrap_or("")
    {
        "uk" => LngList::Uk,
        "ru" => LngList::Ru,
        "zh" => LngList::Zh,
        "tc" => LngList::Tc,
        _ => LngList::En,
    }
}

pub fn load_prefs() -> LngPrefs {
    let mut p = confy::load::<LngPrefs>(APP, CFG).unwrap_or_default();
    if !p.initialized {
        p.lang = detect_os_lang();
        p.initialized = true;
        let _ = confy::store(APP, CFG, &p);
    }
    p
}

// где меняешь язык вручную — просто сохраняй:
pub fn save_lang(lang: LngList) {
    let _ = confy::store(APP, CFG, &LngPrefs { lang, initialized: true });
}
