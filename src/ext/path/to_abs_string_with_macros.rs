use normpath::PathExt as _;
use std::path::Path; // для normalize()

pub trait PathMacrosExt {
    /// Превращает путь в строку:
    /// абсолютный, нормализованный, с заменой HOME → ~ (unix) или USERPROFILE/HOME → %VAR% (windows).
    fn to_abs_string_with_macros(&self) -> String;
}

impl PathMacrosExt for Path {
    fn to_abs_string_with_macros(&self) -> String {
        // 1) абсолютный и очищенный от . / ..
        let abs = self
            .normalize()
            .expect("normalize")
            .into_path_buf();
        let mut s = abs.to_string_lossy().to_string();

        // 2) сократить префикс до ~ / %VAR%
        #[cfg(unix)]
        {
            if let Ok(home) = std::env::var("HOME") {
                if has_prefix_boundary(&s, &home, false) {
                    s = format!("~{}", &s[home.len()..]);
                }
            }
        }

        #[cfg(windows)]
        {
            let mut best: Option<(&'static str, String)> = None;
            for var in ["USERPROFILE", "HOME"] {
                if let Ok(v) = std::env::var(var) {
                    let pref = trim_trailing_bslash(&v.replace('/', "\\"));
                    if has_prefix_boundary(&s, &pref, true)
                        && best
                            .as_ref()
                            .map_or(true, |(_, b)| pref.len() > b.len())
                    {
                        best = Some((var, pref));
                    }
                }
            }
            if let Some((var, pref)) = best {
                s = format!("%{var}%{}", &s[pref.len()..]);
            }
        }

        s
    }
}

#[cfg(windows)]
fn trim_trailing_bslash(mut s: String) -> String {
    while s.ends_with('\\') && s.len() > 1 {
        s.pop();
    }
    s
}

fn has_prefix_boundary(s: &str, pref: &str, ci: bool) -> bool {
    if s.len() < pref.len() {
        return false;
    }
    let (head, tail) = s.split_at(pref.len());
    let eq = if ci { head.eq_ignore_ascii_case(pref) } else { head == pref };
    eq && (tail.is_empty() || matches!(tail.as_bytes()[0], b'/' | b'\\'))
}
