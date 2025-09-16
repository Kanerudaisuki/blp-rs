use crate::err::error::BlpError;
use crate::image_blp::MAX_MIPS;
use crate::ui::viewer::app::App;
use eframe::egui::{ColorImage, Context, TextureOptions, vec2};
use std::sync::mpsc::TryRecvError;

impl App {
    pub(crate) fn poll_decoder(&mut self, ctx: &Context) {
        if !self.loading {
            return;
        }
        // Чтобы UI не "замерзал", просим перерисовку
        ctx.request_repaint();

        // ВАЖНО: сначала вынуть rx из self, чтобы дальше можно было менять self.*
        let Some(rx) = self.decode_rx.take() else {
            // канала нет — считать, что загрузка сорвалась
            self.loading = false;
            return;
        };

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
                self.loading = false;
                // rx дропаем — декодер завершён
            }

            // === ошибка из воркера (AppErr) ===
            Ok(Err(err)) => {
                // Вкладываем как причину в "внешний" ключ, если нужен контекст
                self.error = Some(BlpError::new("error-poll-decoder").push_blp(err));
                self.blp = None;
                self.loading = false;
                // rx дропаем
            }

            // === канал пуст — оставляем rx и ждём следующего кадра ===
            Err(TryRecvError::Empty) => {
                self.decode_rx = Some(rx); // вернуть канал назад
                // self.loading остаётся true
            }

            // === воркер умер — фиксируем явную ошибку ===
            Err(TryRecvError::Disconnected) => {
                self.error = Some(BlpError::new("blp.decode-thread-disconnected").with_arg("msg", "decoder thread disconnected"));
                self.blp = None;
                self.loading = false;
                // rx дропаем
            }
        }
    }
}
