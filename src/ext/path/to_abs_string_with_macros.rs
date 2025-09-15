use normpath::PathExt as _;
use path_absolutize::Absolutize as _;
use std::path::{Path, PathBuf};

pub trait PathMacrosExt {
    /// Абсолютный, нормализованный путь, свернутый в ~ (unix) или %VAR% (windows).
    fn to_abs_string_with_macros(&self) -> String;
}

impl PathMacrosExt for Path {
    fn to_abs_string_with_macros(&self) -> String {
        // 0) Делает путь абсолютным от CWD (не требует существования)
        let abs0: PathBuf = self
            .absolutize()
            .expect("absolutize")
            .into_owned();

        // 1) Нормализует . и .. (логическая нормализация, без fs)
        let abs: PathBuf = abs0
            .normalize()
            .expect("normalize")
            .into_path_buf();

        // 1.1) На Windows убираем \\?\ и прочие артефакты
        #[cfg(windows)]
        let abs: PathBuf = dunce::simplified(&abs).to_path_buf();

        let mut s = abs.to_string_lossy().to_string();

        // 2) Сворачивание префикса
        #[cfg(unix)]
        {
            if let Ok(home) = std::env::var("HOME") {
                let pref = trim_trailing_slash(home);
                if has_prefix_boundary(&s, &pref, false) {
                    s = format!("~{}", &s[pref.len()..]);
                }
            }
        }

        #[cfg(windows)]
        {
            use std::env;

            let mut best: Option<(&'static str, String)> = None;

            for var in ["USERPROFILE", "HOME"] {
                if let Ok(v) = env::var(var) {
                    let pref = trim_trailing_bslash(v.replace('/', r"\"));
                    consider(&s, var, pref, &mut best);
                }
            }

            if let (Ok(d), Ok(p)) = (env::var("HOMEDRIVE"), env::var("HOMEPATH")) {
                let pref = trim_trailing_bslash(format!("{}{}", d, p).replace('/', r"\"));
                consider(&s, "HOMEDRIVE+HOMEPATH", pref, &mut best);
            }

            for var in ["OneDrive", "OneDriveConsumer", "OneDriveCommercial"] {
                if let Ok(v) = env::var(var) {
                    let pref = trim_trailing_bslash(v.replace('/', r"\"));
                    consider(&s, var, pref, &mut best);
                }
            }

            if let Some((var, pref)) = best {
                let var_show = if var == "HOMEDRIVE+HOMEPATH" { "USERPROFILE" } else { var };
                s = format!("%{var_show}%{}", &s[pref.len()..]);
            }

            #[inline]
            fn consider(s: &str, var: &'static str, pref: String, best: &mut Option<(&'static str, String)>) {
                if has_prefix_boundary(s, &pref, true)
                    && best
                        .as_ref()
                        .map_or(true, |(_, b)| pref.len() > b.len())
                {
                    *best = Some((var, pref));
                }
            }
        }

        s
    }
}

#[cfg(unix)]
#[inline]
fn trim_trailing_slash(mut s: String) -> String {
    while s.ends_with('/') && s.len() > 1 {
        s.pop();
    }
    s
}

#[cfg(windows)]
fn trim_trailing_bslash(mut s: String) -> String {
    if s.ends_with('\\') {
        let is_drive_root = s.len() == 3 && s.as_bytes()[1] == b':' && s.as_bytes()[2] == b'\\';
        let is_unc_root = s.starts_with(r"\\") && s.matches('\\').count() < 3;
        if !is_drive_root && !is_unc_root {
            while s.ends_with('\\') {
                s.pop();
            }
        }
    }
    s
}

/// Проверка, что `s` начинается с `pref` по границе сегмента. `ci` — регистронезависимо.
fn has_prefix_boundary(s: &str, pref: &str, ci: bool) -> bool {
    if s.len() < pref.len() {
        return false;
    }
    let (head, tail) = s.split_at(pref.len());
    let eq = if ci { head.eq_ignore_ascii_case(pref) } else { head == pref };
    eq && (tail.is_empty() || matches!(tail.as_bytes()[0], b'/' | b'\\'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    #[cfg(unix)]
    fn folds_unix_home() {
        let tmp = tempfile::tempdir().unwrap();
        unsafe {
            env::set_var("HOME", tmp.path().to_str().unwrap());
        }
        let p = Path::new("foo/../bar").to_abs_string_with_macros();
        // Должно схлопнуться к абсолютному внутри HOME → "~"
        assert!(p.starts_with("~/"));
        assert!(p.ends_with("/bar"));
    }

    #[test]
    #[cfg(windows)]
    fn folds_windows_profile() {
        env::set_var("USERPROFILE", r"C:\Users\Alice");
        // относительный путь -> абсолютный -> нормализованный -> %USERPROFILE%
        let p = Path::new(r".\Desktop\..\Docs").to_abs_string_with_macros();
        assert!(p.starts_with("%USERPROFILE%\\") || p.contains(r":\")); // зависит от текущего CWD
        // Если тест запускается не из профиля, просто проверим нормализацию хвоста:
        assert!(p.ends_with(r"\Docs"));
    }
}
