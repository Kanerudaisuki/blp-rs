use crate::err::error::BlpError;
use crate::image_blp::{ImageBlp, MAX_MIPS};
use crate::ui::i18n::lng_list::LngList;
use crate::ui::i18n::prefs::load_prefs;
use crate::ui::viewer::fonts::install_fonts::install_fonts;
use crate::ui::viewer::layout::file_saver::save_same_dir::save_same_dir_load;
use eframe::egui::{Context, TextureHandle};
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct App {
    pub lng: LngList,
    pub bg_seed: u64,
    pub maximized: bool,
    pub picked_file: Option<PathBuf>,
    pub loading: bool,
    pub error: Option<BlpError>, // один корень ошибки
    pub blp: Option<ImageBlp>,
    pub selected_mip: usize,
    pub mip_textures: Vec<Option<TextureHandle>>, // len == 16
    pub decode_rx: Option<Receiver<Result<ImageBlp, BlpError>>>,
    pub mip_visible: [bool; MAX_MIPS], // init: [true; 16]
    pub save_same_dir: bool,
}

impl App {
    pub fn new(ctx: &Context) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let app = Self {
            lng: load_prefs().lang,
            maximized: false, //
            bg_seed: (nanos as u64) ^ ((nanos >> 64) as u64),
            picked_file: None,
            decode_rx: None,
            loading: false,
            error: None,
            blp: None,
            selected_mip: 0,
            mip_textures: vec![None; MAX_MIPS],
            mip_visible: [true; MAX_MIPS],
            save_same_dir: save_same_dir_load(),
        };
        install_fonts(ctx);
        app
    }
}
