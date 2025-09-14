use fluent_templates::FluentBundle;
use fluent_templates::fluent_bundle::{FluentArgs, FluentError, FluentResource};
use unic_langid::LanguageIdentifier;

/// Универсальный интерфейс локализации.
pub trait Localizer {
    /// Вернуть локализованную строку по ключу + args.
    fn fmt(&self, key: &str, args: &FluentArgs) -> String;
}

/// Обёртка над FluentBundle.
pub struct FtlBundle<'a> {
    pub bundle: FluentBundle<&'a FluentResource>,
}

impl<'a> FtlBundle<'a> {
    pub fn new(lang: LanguageIdentifier, resources: impl IntoIterator<Item = &'a FluentResource>) -> Self {
        let mut bundle = FluentBundle::new_concurrent(vec![lang]);
        for r in resources {
            let _ = bundle.add_resource(r);
        }
        Self { bundle }
    }
}

impl<'a> Localizer for FtlBundle<'a> {
    fn fmt(&self, key: &str, args: &FluentArgs) -> String {
        let msg = match self.bundle.get_message(key) {
            Some(m) => m,
            None => return format!("[missing:{key}]"),
        };
        let pat = match msg.value() {
            Some(v) => v,
            None => return format!("[empty:{key}]"),
        };
        let mut errs: Vec<FluentError> = Vec::new();
        let s = self
            .bundle
            .format_pattern(pat, Some(args), &mut errs);
        if errs.is_empty() {
            s.into_owned()
        } else {
            // В отладке полезно видеть ошибки форматирования
            format!("{s}  /* {:?} */", errs)
        }
    }
}
