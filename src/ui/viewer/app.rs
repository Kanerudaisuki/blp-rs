use crate::image_blp::ImageBlp;
use crate::ui::viewer::fonts::install_fonts::install_fonts;
use crate::ui::viewer::set_initial_file::DecodeResult;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct App {
    pub bg_seed: u64,
    pub maximized: bool,
    pub picked_file: Option<PathBuf>,
    pub loading: bool,
    pub last_err: Option<String>,
    pub is_blp: bool,
    pub blp: Option<ImageBlp>,
    pub selected_mip: usize,
    pub mip_textures: Vec<Option<egui::TextureHandle>>, // len == 16
    pub decode_rx: Option<Receiver<DecodeResult>>,
    pub mip_visible: [bool; 16], // init: [true; 16]
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
            decode_rx: None,
            loading: false,
            last_err: None,
            is_blp: false,
            blp: None,
            selected_mip: 0,
            mip_textures: vec![None; 16],
            mip_visible: [true; 16],
        };
        install_fonts(ctx);
        app
    }
}
