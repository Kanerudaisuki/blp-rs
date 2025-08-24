use crate::ui::viewer::fonts::install_fonts::install_fonts;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct App {
    pub bg_seed: u64,
    pub(crate) maximized: bool,
    pub(crate) picked_file: Option<std::path::PathBuf>,
}

impl App {
    pub fn new(ctx: &egui::Context) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let app = Self {
            maximized: false, //
            bg_seed: (nanos as u64) ^ ((nanos >> 64) as u64),
            picked_file: None,
        };
        install_fonts(ctx);
        app
    }
}
