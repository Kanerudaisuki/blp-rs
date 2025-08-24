use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::mpsc::{self, TryRecvError},
    thread,
};

use egui::{Color32, ColorImage};
use image::RgbaImage;

use crate::ui::viewer::app::App;

impl App {
    /// Точка входа: сохраняем путь и стартуем фоновый декод.
    pub(crate) fn set_initial_file(&mut self, p: Option<PathBuf>) {
        if let Some(path) = p {
            if !path.exists() {
                return;
            }
            self.picked_file = Some(path.clone());
            self.preview_tex = None;
            self.last_err = None;

            let (tx, rx) = mpsc::sync_channel(1);
            self.decode_rx = Some(rx);
            self.loading = true;

            thread::spawn(move || {
                let res = decode_to_rgba_by_magic(&path).map_err(|e| format!("decode error for {:?}: {}", path, e));
                let _ = tx.send(res);
            });
        }
    }

    /// Вызывай каждый кадр в `update`: заберёт результат декодера и создаст текстуру.
    pub(crate) fn poll_decoder(&mut self, ctx: &egui::Context) {
        if !self.loading {
            return;
        }
        ctx.request_repaint();

        if let Some(rx) = &self.decode_rx {
            match rx.try_recv() {
                Ok(Ok(rgba)) => {
                    let ci = rgba_to_color_image(&rgba);
                    self.preview_tex = Some(ctx.load_texture("preview", ci, egui::TextureOptions::LINEAR));
                    self.decode_rx = None;
                    self.loading = false;
                    self.last_err = None;
                }
                Ok(Err(e)) => {
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

// ---------- низкоуровневые помощники (в этом же файле, вне impl) ----------

fn decode_to_rgba_by_magic(path: &Path) -> Result<RgbaImage, Box<dyn std::error::Error + Send + Sync>> {
    if file_starts_with(path, b"BLP")? {
        decode_blp_to_rgba(path)
    } else {
        Ok(image::open(path)?.to_rgba8())
    }
}

fn file_starts_with(path: &Path, magic: &[u8]) -> std::io::Result<bool> {
    let mut f = File::open(path)?;
    let mut buf = [0u8; 4];
    let n = f.read(&mut buf)?;
    Ok(n >= magic.len() && &buf[..magic.len()] == magic)
}

// ← вот сюда подключи свой реальный BLP-декодер и верни RgbaImage
fn decode_blp_to_rgba(path: &Path) -> Result<RgbaImage, Box<dyn std::error::Error + Send + Sync>> {
    // пример: let img: RgbaImage = blp_rs::decode_to_rgba(path)?;
    // Ok(img)
    Err("BLP decoder not wired yet".into())
}

fn rgba_to_color_image(img: &image::RgbaImage) -> ColorImage {
    let (w, h) = img.dimensions();
    let mut pixels = Vec::with_capacity((w * h) as usize);
    for p in img.pixels() {
        pixels.push(Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]));
    }
    ColorImage {
        size: [w as usize, h as usize],
        source_size: egui::vec2(w as f32, h as f32), // 👈 обязателен
        pixels,
    }
}
