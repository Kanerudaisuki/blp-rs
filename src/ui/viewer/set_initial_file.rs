use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::mpsc::{self, TryRecvError},
    thread,
};

use crate::image_blp::{Header, ImageBlp};
use crate::mipmap::Mipmap;
use crate::ui::viewer::app::App;
use egui::ColorImage;
use image::{
    RgbaImage,
    imageops::{FilterType, crop_imm, resize},
};

/// Сообщение из фонового декодера: единый путь — всегда ImageBlp или ошибка
pub enum DecodeResult {
    Blp(ImageBlp),
    Err(String),
}

impl App {
    /// Сохраняем путь и стартуем фоновый декод.
    pub(crate) fn set_initial_file(&mut self, p: Option<PathBuf>) {
        if let Some(path) = p {
            if !path.exists() {
                return;
            }
            self.picked_file = Some(path.clone());
            self.last_err = None;

            // быстрый флаг для UI
            self.is_blp = file_starts_with(&path, b"BLP").unwrap_or(false);

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

    /// Забирает результат декодера и создаёт текстуры.
    pub(crate) fn poll_decoder(&mut self, ctx: &egui::Context) {
        if !self.loading {
            return;
        }
        ctx.request_repaint();

        if let Some(rx) = &self.decode_rx {
            match rx.try_recv() {
                Ok(DecodeResult::Blp(mut blp)) => {
                    // НОРМАЛИЗАЦИЯ ТОЛЬКО ЗДЕСЬ: ровно 16 слотов (пустые = 0×0, None)
                    normalize_blp_to_16_slots(&mut blp);

                    // Заливка текстур только для существующих уровней с картинкой
                    for (i, m) in blp.mipmaps.iter().enumerate().take(16) {
                        if let Some(img) = &m.image {
                            let (w, h) = (m.width as usize, m.height as usize);
                            let mut ci = ColorImage::from_rgba_unmultiplied([w, h], img.as_raw());
                            ci.source_size = egui::vec2(w as f32, h as f32);
                            self.mip_textures[i] = Some(ctx.load_texture(format!("mip_{i}"), ci, egui::TextureOptions::LINEAR));
                        } else {
                            self.mip_textures[i] = None;
                        }
                    }

                    self.selected_mip = (0..16)
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

// ---------- низкоуровневые помощники ----------

fn decode_by_magic(path: &Path) -> DecodeResult {
    match file_starts_with(path, b"BLP") {
        Ok(true) => match decode_blp(path) {
            Ok(blp) => {
                // BLP-декодер не выдаёт нулевые размеры — не правим
                if blp.mipmaps.is_empty() {
                    return DecodeResult::Err("empty BLP mip chain".into());
                }
                DecodeResult::Blp(blp)
            }
            Err(e) => DecodeResult::Err(format!("BLP decode failed: {e}")),
        },
        _ => match image::open(path) {
            Ok(img_dyn) => {
                // Растр: проверка на нулевые стороны → ошибка
                let base = img_dyn.to_rgba8();
                let (bw, bh) = base.dimensions();
                if bw == 0 || bh == 0 {
                    return DecodeResult::Err("raster image has zero width or height".into());
                }

                // Центр-кроп до степеней двойки, затем мип-цепочка «пополам до min(side)=1»
                let cropped = center_crop_to_pow2(&base);
                let chain = build_mips_halving_until_one(&cropped);

                let mut mipmaps: Vec<Mipmap> = chain
                    .into_iter()
                    .map(|im| {
                        let (w, h) = im.dimensions();
                        Mipmap { width: w, height: h, image: Some(im) }
                    })
                    .collect();

                // Добиваем список до 16 слотов пустыми (0×0, None)
                pad_to_16_slots(&mut mipmaps);

                let (w0, h0) = mipmaps
                    .get(0)
                    .map(|m| (m.width, m.height))
                    .unwrap_or((0, 0));

                let blp = ImageBlp { header: Header { width: w0, height: h0, ..Default::default() }, mipmaps, holes: 0 };
                DecodeResult::Blp(blp)
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
fn decode_blp(path: &Path) -> Result<ImageBlp, Box<dyn std::error::Error + Send + Sync>> {
    // читаем весь файл в память
    let data = std::fs::read(path)?;

    // парсим bytes -> ImageBlp
    let blp = ImageBlp::from_bytes(&data).map_err(|e| {
        // заворачиваем в io::Error, чтобы привести к Box<dyn Error + Send + Sync>
        std::io::Error::new(std::io::ErrorKind::InvalidData, format!("from_bytes failed: {e}"))
    })?;

    Ok(blp)
}

// ----- растр: центр-кроп до степеней двойки + мип-цепочка -----

#[inline]
fn floor_pow2(x: u32) -> u32 {
    1 << (31 - x.max(1).leading_zeros())
}

fn center_crop_to_pow2(src: &RgbaImage) -> RgbaImage {
    let (w0, h0) = src.dimensions();
    // сюда приходим только с w0>0 && h0>0 (проверено выше)
    let w1 = floor_pow2(w0);
    let h1 = floor_pow2(h0);
    let x0 = (w0 - w1) / 2;
    let y0 = (h0 - h1) / 2;
    crop_imm(src, x0, y0, w1, h1).to_image()
}

/// Делим пополам, пока ОДНА из сторон не станет 1. Макс 16 уровней.
/// Для квадратов (2^n × 2^n) последний уровень будет 1×1 — это НОРМАЛЬНЫЙ реальный mip,
/// но пустых 1×1-заглушек мы НЕ добавляем.
fn build_mips_halving_until_one(base: &RgbaImage) -> Vec<RgbaImage> {
    let (mut w, mut h) = base.dimensions();
    let mut out = Vec::with_capacity(16);
    out.push(base.clone());

    while (w > 1 && h > 1) && out.len() < 16 {
        let nw = (w / 2).max(1);
        let nh = (h / 2).max(1);
        let next = resize(out.last().unwrap(), nw, nh, FilterType::Triangle);
        out.push(next);
        w = nw;
        h = nh;
        if w == 1 || h == 1 {
            break; // стоп на 2×1 / 1×N или естественный 1×1 (из квадрата)
        }
    }
    out
}

/// Добиваем список до ровно 16 элементов ПУСТЫМИ слотами: width=0, height=0, image=None.
fn pad_to_16_slots(mips: &mut Vec<Mipmap>) {
    if mips.len() > 16 {
        mips.truncate(16);
        return;
    }
    while mips.len() < 16 {
        mips.push(Mipmap { width: 0, height: 0, image: None });
    }
}

// ----- BLP: довести список до ровно 16 слотов (пустые 0×0), без дорисовки 1×1 -----

fn normalize_blp_to_16_slots(blp: &mut ImageBlp) {
    if blp.mipmaps.len() > 16 {
        blp.mipmaps.truncate(16);
    }
    while blp.mipmaps.len() < 16 {
        blp.mipmaps
            .push(Mipmap { width: 0, height: 0, image: None });
    }
}
