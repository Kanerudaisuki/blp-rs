use crate::export::last_dir::{load_last_dir, save_last_dir};
use crate::ui::viewer::update::App;
use std::path::PathBuf;

impl App {
    pub fn default_names(&self) -> (String, String) {
        let stem = self
            .current_path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "texture".to_string());
        (format!("{stem}.blp"), format!("{stem}.png"))
    }

    /// Открывает диалог "Save as..." с общей «последней папкой».
    pub fn pick_save_path(&mut self, default_name: &str, ext: &str, desc: &str) -> Option<PathBuf> {
        let mut dlg = rfd::FileDialog::new()
            .set_file_name(default_name)
            .add_filter(desc, &[ext]);

        // приоритет: сохранённая папка → папка текущего файла → дефолт ОС
        if let Some(dir) = load_last_dir().or_else(|| {
            self.current_path
                .as_ref()
                .and_then(|p| p.parent().map(|pp| pp.to_path_buf()))
        }) {
            dlg = dlg.set_directory(dir);
        }

        let path = dlg.save_file()?;
        if let Some(parent) = path.parent() {
            let _ = save_last_dir(parent); // best-effort
        }
        Some(path)
    }
}
