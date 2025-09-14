use crate::decode::decode_input::{DecodeInput, decode_input};
use crate::err::app_err::AppErr;
use crate::ui::viewer::app::App;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

impl App {
    pub(crate) fn pick_from_file(&mut self, p: Option<PathBuf>) {
        if let Some(path) = p {
            self.error = if path.exists() {
                None
            } else {
                Some(AppErr::new("file-not-found").with_arg("path", path.to_string_lossy().into_owned()))
            };

            if self.error.is_some() {
                return;
            }

            self.picked_file = Some(path.clone());
            self.blp = None;
            self.selected_mip = 0;
            self.mip_textures.fill_with(|| None);

            let (tx, rx) = mpsc::sync_channel(1);
            self.decode_rx = Some(rx);
            self.loading = true;

            thread::spawn(move || {
                let res = decode_input(DecodeInput::Path(path));
                let _ = tx.send(res);
            });
        }
    }
}
