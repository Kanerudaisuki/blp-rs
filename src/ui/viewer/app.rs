use crate::decode::decode_result::DecodeResult;
use crate::image_blp::{ImageBlp, MAX_MIPS};
use crate::ui::i18n::lng_list::LngList;
use crate::ui::i18n::prefs::load_prefs;
use crate::ui::viewer::fonts::install_fonts::install_fonts;
use eframe::egui::{Context, TextureHandle};
use std::fmt::Display;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct App {
    pub lng: LngList,
    pub bg_seed: u64,
    pub maximized: bool,
    pub picked_file: Option<PathBuf>,
    pub loading: bool,
    pub err: Option<String>,
    pub blp: Option<ImageBlp>,
    pub selected_mip: usize,
    pub mip_textures: Vec<Option<TextureHandle>>, // len == 16
    pub decode_rx: Option<Receiver<DecodeResult>>,
    pub mip_visible: [bool; MAX_MIPS], // init: [true; 16]
    pub current_path: Option<PathBuf>, // откуда файл открыт (если есть)
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
            err: None,
            blp: None,
            selected_mip: 0,
            mip_textures: vec![None; MAX_MIPS],
            mip_visible: [true; MAX_MIPS],
            current_path: None,
        };
        install_fonts(ctx);
        app
    }

    #[inline]
    pub fn err_set<E: Display>(&mut self, err: E) {
        self.err = Some(err.to_string());
    }

    #[inline]
    pub fn err_clear(&mut self) {
        self.err = None;
    }
}
