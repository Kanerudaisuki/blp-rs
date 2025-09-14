use crate::err::args::ArgVal;
use crate::err::localize::Localizer;
use fluent_templates::fluent_bundle::FluentArgs;
use std::error::Error;
use std::{collections::BTreeMap, fmt, sync::Arc};

#[derive(Debug, Clone)]
pub struct AppErr {
    pub key: &'static str,
    pub args: BTreeMap<&'static str, ArgVal>,
    pub causes: Vec<Cause>,
}

#[derive(Debug, Clone)]
pub enum Cause {
    App(AppErr),
    Std(Arc<dyn Error + Send + Sync>),
}

pub type AppResult<T> = Result<T, AppErr>;

impl AppErr {
    /// Обернуть существующую ошибку контекстом (ключом), вложив её как причину.
    #[inline]
    pub fn ctx(self, key: &'static str) -> AppErr {
        AppErr::new(key).push_app(self)
    }

    /// То же, но с добавлением аргументов.
    #[inline]
    pub fn ctx_with(self, key: &'static str, f: impl FnOnce(AppErr) -> AppErr) -> AppErr {
        f(AppErr::new(key).push_app(self))
    }

    #[inline]
    pub fn new(key: &'static str) -> Self {
        Self { key, args: BTreeMap::new(), causes: Vec::new() }
    }

    #[inline]
    pub fn with_arg(mut self, name: &'static str, val: impl Into<ArgVal>) -> Self {
        self.args.insert(name, val.into());
        self
    }

    #[inline]
    pub fn with_args(mut self, args: impl IntoIterator<Item = (&'static str, ArgVal)>) -> Self {
        for (k, v) in args {
            self.args.insert(k, v);
        }
        self
    }

    #[inline]
    pub fn push_app(mut self, cause: AppErr) -> Self {
        self.causes.push(Cause::App(cause));
        self
    }

    #[inline]
    pub fn push_std(mut self, cause: impl Error + Send + Sync + 'static) -> Self {
        self.causes
            .push(Cause::Std(Arc::new(cause)));
        self
    }

    /// Локализованный текст этой ошибки (без детей).
    pub fn localized_head<L: Localizer>(&self, loc: &L) -> String {
        let mut fluent_args = FluentArgs::new();
        for (k, v) in &self.args {
            fluent_args.set(*k, v.to_fluent_owned());
        }
        loc.fmt(self.key, &fluent_args)
    }
}

impl fmt::Display for AppErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Быстрая небогатая форма (ключ + перечисление args)
        write!(f, "{}(", self.key)?;
        let mut first = true;
        for (k, v) in &self.args {
            if !first {
                write!(f, ", ")?;
            }
            first = false;
            write!(f, "{k}={v:?}")?;
        }
        write!(f, ")")
    }
}

impl<E> From<E> for AppErr
where
    E: Error + Send + Sync + 'static,
{
    fn from(e: E) -> Self {
        AppErr::new("std-error")
            .with_arg("msg", e.to_string())
            .push_std(e)
    }
}
