use crate::err::app_err::{AppErr, Cause};
use crate::err::localize::Localizer;
use std::fmt::{self};

#[derive(Copy, Clone, Debug)]
pub enum TreeFmtMode {
    Plain,     // ключ + args (Display)
    Localized, // через Localizer
}

pub struct TreeFmt<'a, L> {
    pub root: &'a AppErr,
    pub mode: TreeFmtMode,
    pub loc: Option<&'a L>,
}

impl<'a, L: Localizer> TreeFmt<'a, L> {
    pub fn localized(root: &'a AppErr, loc: &'a L) -> Self {
        Self { root, mode: TreeFmtMode::Localized, loc: Some(loc) }
    }
}

impl<'a, L: Localizer> fmt::Display for TreeFmt<'a, L> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn write_line<L: Localizer>(f: &mut fmt::Formatter<'_>, err: &AppErr, mode: TreeFmtMode, loc: Option<&L>, indent: usize) -> fmt::Result {
            for _ in 0..indent {
                f.write_str("  ")?;
            }
            match mode {
                TreeFmtMode::Plain => writeln!(f, "{err}")?,
                TreeFmtMode::Localized => {
                    let Some(loc) = loc else {
                        writeln!(f, "{err}")?;
                        return Ok(());
                    };
                    writeln!(f, "{}", err.localized_head(loc))?
                }
            }
            for c in &err.causes {
                match c {
                    Cause::App(a) => write_line(f, a, mode, loc, indent + 1)?,
                    Cause::Std(e) => {
                        for _ in 0..(indent + 1) {
                            f.write_str("  ")?;
                        }
                        writeln!(f, "{e}")?;
                    }
                }
            }
            Ok(())
        }

        write_line(f, self.root, self.mode, self.loc, 0)
    }
}
