use crate::err::args::ArgVal;
use crate::err::cause::Cause;
use std::error::Error;
use std::{collections::BTreeMap, fmt, sync::Arc};

#[derive(Debug, Clone)]
pub struct BlpErr {
    pub key: &'static str,
    pub args: BTreeMap<&'static str, ArgVal>,
    pub causes: Vec<Cause>,
}

impl BlpErr {
    /// Обернуть существующую ошибку контекстом (ключом), вложив её как причину.
    #[inline]
    pub fn ctx(self, key: &'static str) -> BlpErr {
        BlpErr::new(key).push_blp(self)
    }

    /// То же, но с добавлением аргументов.
    #[inline]
    pub fn ctx_with(self, key: &'static str, f: impl FnOnce(BlpErr) -> BlpErr) -> BlpErr {
        f(BlpErr::new(key).push_blp(self))
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
    pub fn push_blp(mut self, cause: BlpErr) -> Self {
        self.causes.push(Cause::Blp(cause));
        self
    }

    #[inline]
    pub fn push_std(mut self, cause: impl Error + Send + Sync + 'static) -> Self {
        self.causes
            .push(Cause::Std(Arc::new(cause)));
        self
    }
}

impl fmt::Display for BlpErr {
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

impl<E> From<E> for BlpErr
where
    E: Error + Send + Sync + 'static,
{
    fn from(e: E) -> Self {
        BlpErr::new("std-error")
            .with_arg("msg", e.to_string())
            .push_std(e)
    }
}
