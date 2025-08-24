use crate::ui::viewer::fonts::install_fonts::install_fonts;
use image::RgbaImage;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct App {
    pub bg_seed: u64,
    pub(crate) maximized: bool,
    pub(crate) picked_file: Option<PathBuf>,
    // ...
    pub(crate) preview_tex: Option<egui::TextureHandle>,
    pub(crate) decode_rx: Option<Receiver<Result<RgbaImage, String>>>,
    pub(crate) loading: bool,
    pub(crate) last_err: Option<String>,
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
            preview_tex: None,
            decode_rx: None,
            loading: false,
            last_err: None,
        };
        install_fonts(ctx);
        app
    }
}
