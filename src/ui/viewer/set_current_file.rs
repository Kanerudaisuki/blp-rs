use crate::decode::decode_result::DecodeResult;
use std::{
    path::PathBuf,
    sync::mpsc::{self, TryRecvError},
    thread,
};

use crate::decode::decode_input::{DecodeInput, decode_input};
use crate::ui::viewer::app::App;
use egui::ColorImage;
use crate::image_blp::MAX_MIPS;

impl App {
    /// Сохраняем путь и стартуем фоновый декод.
    pub(crate) fn set_current_file(&mut self, p: Option<PathBuf>) {
        if let Some(path) = p {
            if !path.exists() {
                return;
            }

            self.picked_file = Some(path.clone());
            self.last_err = None;
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

    /// Забирает результат декодера и создаёт текстуры.
    pub(crate) fn poll_decoder(&mut self, ctx: &egui::Context) {
        if !self.loading {
            return;
        }
        ctx.request_repaint();

        if let Some(rx) = &self.decode_rx {
            match rx.try_recv() {
                Ok(DecodeResult::Blp(blp)) => {
                    // Заливка текстур только для существующих уровней с картинкой
                    for (i, m) in blp.mipmaps.iter().enumerate().take(MAX_MIPS) {
                        if let Some(img) = &m.image {
                            let (w, h) = (m.width as usize, m.height as usize);
                            let mut ci = ColorImage::from_rgba_unmultiplied([w, h], img.as_raw());
                            ci.source_size = egui::vec2(w as f32, h as f32);
                            self.mip_textures[i] = Some(ctx.load_texture(format!("mip_{i}"), ci, egui::TextureOptions::LINEAR));
                        } else {
                            self.mip_textures[i] = None;
                        }
                    }

                    self.selected_mip = (0..MAX_MIPS)
                        .find(|&i| self.mip_textures[i].is_some())
                        .unwrap_or(0);

                    self.blp = Some(blp);
                    self.decode_rx = None;
                    self.loading = false;
                    self.last_err = None;
                }
                Ok(DecodeResult::Err(e)) => {
                    self.last_err = Some(e);
                    self.decode_rx = None;
                    self.loading = false;
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    self.last_err = Some("decoder thread disconnected".into());
                    self.decode_rx = None;
                    self.loading = false;
                }
            }
        }
    }
}
