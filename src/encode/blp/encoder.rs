// src/encode/blp/encoder.rs
use crate::encode::blp::ctx::ctx::EncoderCtx;
use crate::encode::blp::jpeg::parse::extract_plan;
use crate::encode::blp::options::EncoderOptions;
// no plan needed for header+scan path
use crate::err::error::BlpError;
use crate::image_blp::ImageBlp;
use std::time::Instant;

impl ImageBlp {
    pub fn encode_blp(&self, opts: EncoderOptions) -> Result<EncoderCtx, BlpError> {
        println!("== encode_blp: start ==");

        // A) layout + ctx (без JPEG)
        let mut ctx = self.build_layout_ctx(opts)?;
        println!("base(after shift): {}x{}, has_alpha={}, visible_count={}", ctx.base_width, ctx.base_height, ctx.has_alpha, ctx.visible_count);

        // B) кодирование всех включённых мипов high-level (полный JPEG на мип)
        let first_idx = ctx
            .mips
            .iter()
            .position(|m| m.included)
            .ok_or_else(|| BlpError::new("encode_blp_no_visible"))?;
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
            let scan_bytes = u
                .jpeg_slices
                .as_ref()
                .map(|s| s.scan_len)
                .unwrap_or_default();
            println!("mip{}: {}x{} -> {} bytes (scan {}); t_acc={:.3} ms", u.index, u.width, u.height, u.jpeg_full_bytes, scan_bytes, t_acc_ms);
        }

        // Общий план: если JPEG baseline, строим компактный заголовок; иначе возьмём header mip0 до SOS
        let plan_opt = extract_plan(&ctx.mips[first_idx].jpeg_full).ok();
        ctx.jpeg_plan = plan_opt.clone();

        let common = if let Some(plan) = &ctx.jpeg_plan {
            crate::encode::blp::jpeg::build::build_common_header(plan)?
        } else {
            let first = ctx
                .mips
                .iter()
                .find(|m| m.included)
                .ok_or_else(|| BlpError::new("encode_blp_no_visible"))?;
            let slices0 = first
                .jpeg_slices
                .ok_or_else(|| BlpError::new("encode_blp_missing_slices").with_arg("mip", first.index as u32))?;
            first.jpeg_full[..slices0.head_len].to_vec()
        };
        ctx.common_header_len = common.len();
        ctx.common_header = common;

        ctx.alpha_depth = if ctx.has_alpha { 8 } else { 0 };

        // D) пишем контейнер: common header + только SCAN каждого мипа (+ EOI)
        ctx.write_blp_common_header_and_slices()?;
        Ok(ctx)
    }
}
