use crate::image_blp::MAX_MIPS;
use crate::ui::viewer::app::App;
use eframe::egui::{ColorImage, Context, TextureOptions, vec2};
use std::sync::mpsc::TryRecvError;

impl App {
    pub(crate) fn poll_decoder(&mut self, ctx: &Context) {
        if !self.loading {
            return;
        }
        ctx.request_repaint();

        if let Some(rx) = &self.decode_rx {
            match rx.try_recv() {
                // === успех ===
                Ok(Ok(blp)) => {
                    // Заливка текстур только для существующих уровней
                    for (i, m) in blp
                        .mipmaps
                        .iter()
                        .enumerate()
                        .take(MAX_MIPS)
                    {
                        if let Some(img) = &m.image {
                            let (w, h) = (m.width as usize, m.height as usize);
                            let mut ci = ColorImage::from_rgba_unmultiplied([w, h], img.as_raw());
                            ci.source_size = vec2(w as f32, h as f32);
                            self.mip_textures[i] = Some(ctx.load_texture(format!("mip_{i}"), ci, TextureOptions::LINEAR));
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
                }

                // === ошибка из воркера (ErrWire) ===
                Ok(Err(wire)) => {
                    //self.errors.push(ErrItem::from_wire(wire));
                    self.decode_rx = None;
                    self.loading = false;
                }

                // канал пуст — ждём дальше
                Err(TryRecvError::Empty) => {}

                // воркер умер — запишем явную ошибку
                Err(TryRecvError::Disconnected) => {
                    //self.errors.push(ErrItem::new(ErrKind::Unknown { msg: "decoder thread disconnected".to_string() }, file!(), line!()));
                    self.decode_rx = None;
                    self.loading = false;
                }
            }
        }
    }
}
