use crate::ui::viewer::app::App;
use crate::ui::viewer::layout::file_saver::last_safe_dir::{last_save_dir_load, last_save_dir_save};
use std::path::PathBuf;

impl App {
    /// Выбор пути сохранения: рядом с исходником (если разрешено и есть picked_file),
    /// иначе показываем диалог и запоминаем выбранную папку в confy.
    pub fn pick_save_path(&mut self, default_name: &str, ext: &str, desc: &str) -> Option<PathBuf> {
        // 1) Режим "рядом" возможен только если известен исходный файл
        if self.save_same_dir {
            if let Some(src) = self.picked_file.as_ref() {
                if let Some(parent) = src.parent() {
                    let file_name = ensure_ext(default_name, ext);
                    return Some(parent.join(file_name));
                }
            }
            // если parent неизвестен — падём в диалог
        }

        // 2) Диалог "Сохранить как…"
        let mut dlg = rfd::FileDialog::new()
            .set_file_name(ensure_ext(default_name, ext))
            .add_filter(desc, &[trim_dot(ext)]);

        // дефолтная папка: confy → родитель picked_file
        if let Some(dir) = last_save_dir_load().or_else(|| {
            self.picked_file
                .as_ref()
                .and_then(|p| p.parent().map(|pp| pp.to_path_buf()))
        }) {
            dlg = dlg.set_directory(dir);
        }

        let path = dlg.save_file()?;

        if let Some(parent) = path.parent() {
            let _ = last_save_dir_save(parent); // best-effort
        }
        Some(ensure_path_has_ext(path, ext))
    }
}

// ----- мелкие утилиты -----

fn ensure_ext(name: &str, ext: &str) -> String {
    let ext = trim_dot(ext);
    if name
        .rsplit('.')
        .next()
        .map(|e| e.eq_ignore_ascii_case(ext))
        .unwrap_or(false)
    {
        name.to_owned()
    } else {
        format!("{name}.{}", ext)
    }
}

fn ensure_path_has_ext(mut p: PathBuf, ext: &str) -> PathBuf {
    let ext = trim_dot(ext);
    let has = p
        .extension()
        .map(|e| e.eq_ignore_ascii_case(ext))
        .unwrap_or(false);
    if !has && p.extension().is_none() {
        p.set_extension(ext);
    }
    p
}

fn trim_dot(ext: &str) -> &str {
    ext.strip_prefix('.').unwrap_or(ext)
}
