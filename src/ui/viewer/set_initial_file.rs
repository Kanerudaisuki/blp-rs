use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::mpsc::{self, TryRecvError},
    thread,
};

use crate::image_blp::ImageBlp;
use crate::mipmap::Mipmap;
use crate::ui::viewer::app::App;
use egui::ColorImage;
use image::{
    RgbaImage,
    imageops::{FilterType, crop_imm, resize},
};

// Сообщение из фонового декодера
pub enum DecodeResult {
    Blp(ImageBlp),
    RasterMips(Vec<Mipmap>),
    Err(String),
}

impl App {
    /// Точка входа: сохраняем путь и стартуем фоновый декод.
    pub(crate) fn set_initial_file(&mut self, p: Option<PathBuf>) {
        if let Some(path) = p {
            if !path.exists() {
                return;
            }
            self.picked_file = Some(path.clone());
            self.last_err = None;
            self.is_blp = false;
            self.blp = None;
            self.selected_mip = 0;
            self.mip_textures.fill_with(|| None);

            let (tx, rx) = mpsc::sync_channel(1);
            self.decode_rx = Some(rx);
            self.loading = true;

            thread::spawn(move || {
                let res = decode_by_magic(&path);
                let _ = tx.send(res);
            });
        }
    }

    /// Вызывай каждый кадр в `update`: заберёт результат декодера и создаст текстуры.
    pub(crate) fn poll_decoder(&mut self, ctx: &egui::Context) {
        if !self.loading {
            return;
        }
        ctx.request_repaint();

        if let Some(rx) = &self.decode_rx {
            match rx.try_recv() {
                Ok(DecodeResult::Blp(mut blp)) => {
                    self.is_blp = true;
                    normalize_blp_to_16_mips(&mut blp);

                    // создать/обновить текстуры мипов
                    for (i, m) in blp.mipmaps.iter().enumerate().take(16) {
                        self.mip_textures[i] = m.image.as_ref().map(|img| {
                            let mut ci = ColorImage::from_rgba_unmultiplied([m.width as usize, m.height as usize], img.as_raw());
                            ci.source_size = egui::vec2(m.width as f32, m.height as f32);
                            ctx.load_texture(format!("blp_mip_{i}"), ci, egui::TextureOptions::LINEAR)
                        });
                    }
                    self.selected_mip = (0..16)
                        .find(|&i| self.mip_textures[i].is_some())
                        .unwrap_or(0);

                    self.blp = Some(blp);
                    self.decode_rx = None;
                    self.loading = false;
                    self.last_err = None;
                }
                Ok(DecodeResult::RasterMips(mips)) => {
                    self.is_blp = false;
                    for (i, m) in mips.iter().enumerate().take(16) {
                        self.mip_textures[i] = m.image.as_ref().map(|img| {
                            let mut ci = ColorImage::from_rgba_unmultiplied([m.width as usize, m.height as usize], img.as_raw());
                            ci.source_size = egui::vec2(m.width as f32, m.height as f32);
                            ctx.load_texture(format!("ras_mip_{i}"), ci, egui::TextureOptions::LINEAR)
                        });
                    }
                    self.selected_mip = (0..16)
                        .find(|&i| self.mip_textures[i].is_some())
                        .unwrap_or(0);

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

// ---------- низкоуровневые помощники (в этом же файле, вне impl) ----------

fn decode_by_magic(path: &Path) -> DecodeResult {
    match file_starts_with(path, b"BLP") {
        Ok(true) => match decode_blp(path) {
            Ok(mut blp) => {
                normalize_blp_to_16_mips(&mut blp);
                DecodeResult::Blp(blp)
            }
            Err(e) => DecodeResult::Err(format!("BLP decode failed: {e}")),
        },
        _ => match image::open(path) {
            Ok(img_dyn) => {
                let base = img_dyn.to_rgba8();
                let cropped = center_crop_to_pow2(&base);
                let chain = build_mips_halving_until_one(&cropped);
                let mut mips: Vec<Mipmap> = chain
                    .into_iter()
                    .map(|im| {
                        let (w, h) = im.dimensions();
                        Mipmap { width: w, height: h, image: Some(im) }
                    })
                    .collect();
                pad_to_16_mips(&mut mips);
                DecodeResult::RasterMips(mips)
            }
            Err(e) => DecodeResult::Err(format!("raster decode failed: {e}")),
        },
    }
}

fn file_starts_with(path: &Path, magic: &[u8]) -> std::io::Result<bool> {
    let mut f = File::open(path)?;
    let mut buf = [0u8; 4];
    let n = f.read(&mut buf)?;
    Ok(n >= magic.len() && &buf[..magic.len()] == magic)
}

// ← подключи сюда свой реальный BLP-декодер
fn decode_blp(_path: &Path) -> Result<ImageBlp, Box<dyn std::error::Error + Send + Sync>> {
    // пример:
    // let blp = blp_rs::read_image(_path)?;
    // Ok(blp)
    Err("TODO: wire your BLP decoder".into())
}

// ----- Не-BLP: центр-кроп до степеней двойки + мип-цепочка -----

#[inline]
fn floor_pow2(x: u32) -> u32 {
    1 << (31 - x.leading_zeros())
}

fn center_crop_to_pow2(src: &RgbaImage) -> RgbaImage {
    let (w0, h0) = src.dimensions();
    let w1 = floor_pow2(w0.max(1));
    let h1 = floor_pow2(h0.max(1));
    let x0 = (w0 - w1) / 2;
    let y0 = (h0 - h1) / 2;
    crop_imm(src, x0, y0, w1, h1).to_image()
}

fn build_mips_halving_until_one(base: &RgbaImage) -> Vec<RgbaImage> {
    let (mut w, mut h) = base.dimensions();
    let mut out = Vec::with_capacity(16);
    out.push(base.clone());
    while w > 1 && h > 1 {
        let nw = (w / 2).max(1);
        let nh = (h / 2).max(1);
        let next = resize(out.last().unwrap(), nw, nh, FilterType::Triangle);
        out.push(next);
        w = nw;
        h = nh;
        if w == 1 || h == 1 {
            break;
        }
    }
    out
}

fn pad_to_16_mips(mips: &mut Vec<Mipmap>) {
    while mips.len() < 16 {
        let (w, h) = if let Some(last) = mips.last() { (last.width.max(1), last.height.max(1)) } else { (1, 1) };
        let nw = (w / 2).max(1);
        let nh = (h / 2).max(1);
        mips.push(Mipmap { width: nw, height: nh, image: None });
    }
    if mips.len() > 16 {
        mips.truncate(16);
    }
}

// ----- BLP: довести список мипов до ровно 16 уровней -----

fn normalize_blp_to_16_mips(blp: &mut ImageBlp) {
    // если пусто — делать нечего
    if blp.mipmaps.is_empty() {
        blp.mipmaps = vec![Mipmap { width: 1, height: 1, image: None }];
    }
    while blp.mipmaps.len() < 16 {
        let (pw, ph) = {
            let last = blp.mipmaps.last().unwrap();
            (last.width.max(1), last.height.max(1))
        };
        let nw = (pw / 2).max(1);
        let nh = (ph / 2).max(1);
        blp.mipmaps
            .push(Mipmap { width: nw, height: nh, image: None });
    }
    if blp.mipmaps.len() > 16 {
        blp.mipmaps.truncate(16);
    }
}
