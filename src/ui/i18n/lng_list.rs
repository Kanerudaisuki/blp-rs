use fluent_templates::{LanguageIdentifier, langid};
use serde::{Deserialize, Serialize};

// Удобный enum для UI
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LngList {
    #[default]
    En,
    Ru,
    Uk,
    Zh,
}

impl LngList {
    pub fn id(self) -> LanguageIdentifier {
        match self {
            LngList::En => langid!("en"),
            LngList::Ru => langid!("ru"),
            LngList::Uk => langid!("uk"),
            LngList::Zh => langid!("zh"),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            LngList::En => "English",
            LngList::Ru => "Русский",
            LngList::Uk => "Українська",
            LngList::Zh => "中文",
        }
    }
}
