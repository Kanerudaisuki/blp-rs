// src/encode/blp/encoder.rs

use crate::encode::blp::ctx::ctx::EncoderCtx;
use crate::encode::blp::options::EncoderOptions;
use crate::err::error::BlpError;
use crate::image_blp::ImageBlp;
use std::time::Instant;

impl ImageBlp {
    /// Полный пайплайн:
    ///  1) лэйаут и контекст (`build_layout_ctx`) — со смещением к первому present+visible,
    ///  2) кодирование только включённых юнитов (`ctx.mips`),
    ///  3) запись контейнера (`ctx.write_blp_full_jpegs`).
    ///
    /// На выходе — заполненный `EncoderCtx` или ошибка.
    pub fn encode_blp(&self, opts: EncoderOptions) -> Result<EncoderCtx, BlpError> {
        println!("== encode_blp: start ==");

        // A) Контекст + юниты (без JPEG)
        let mut ctx = self.build_layout_ctx(opts)?;
        println!("base(after shift): {}x{}, has_alpha={}, visible_count={}", ctx.base_width, ctx.base_height, ctx.has_alpha, ctx.visible_count);

        // B) Кодирование включенных мипов
        let mut t_acc_ms = 0.0_f64;
        for u in &mut ctx.mips {
            if !u.included {
                continue;
            }
            let Some(src) = u.src_index else {
                continue;
            };
            let Some(img) = self.mipmaps[src].image.as_ref() else {
                continue;
            };

            let t0 = Instant::now();
            u.encode_to_jpeg(img, ctx.has_alpha, ctx.quality)?;
            t_acc_ms += t0.elapsed().as_secs_f64() * 1000.0;
            u.encode_ms_acc = t_acc_ms;

            println!("mip{}: {}x{} -> {} bytes; t_acc={:.3} ms", u.index, u.width, u.height, u.jpeg_full_bytes, t_acc_ms);
        }

        ctx.write_blp_full_jpegs()?;
        Ok(ctx)
    }
}
