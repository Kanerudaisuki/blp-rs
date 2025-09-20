use std::time::Instant;

use crate::encode::blp::options::{EncoderCtx, EncoderOptions};
use crate::encode::blp::report::EncodeReport;
use crate::encode::blp::unit::unit::{MipSkipReason, MipUnit};
use crate::encode::blp::writer::write_blp_full_jpegs;
use crate::err::error::BlpError;
use crate::image_blp::{ImageBlp, MAX_MIPS};

impl ImageBlp {
    /// Step 1: encode **FULL** JPEGs for all visible & present mips.
    /// No common header extraction yet. Verbose logging.
    pub fn encode_blp(&self, opts: EncoderOptions) -> Result<EncodeReport, BlpError> {
        println!("== encode_blp: start ==");

        // 0) build context
        let ctx: EncoderCtx = opts.make_ctx(self)?;
        println!("base: {}x{}, first_visible={}, visible_count={}, has_alpha={}", ctx.base_width, ctx.base_height, ctx.first_visible_mip, ctx.visible_count, ctx.has_alpha);

        // 1) fixed-length vector (MAX_MIPS)
        let mut units: Vec<MipUnit> = (0..MAX_MIPS)
            .map(MipUnit::empty)
            .collect();

        // 2) encode present+visible mips
        let mut t_acc_ms = 0.0_f64;

        for i in 0..MAX_MIPS {
            let maybe_mip = self.mipmaps.get(i);
            let is_present = maybe_mip
                .and_then(|m| m.image.as_ref())
                .is_some();
            let is_visible = ctx.eff_visible[i];

            if !is_present {
                // no mip -> mark as NotPresent
                units[i].included = false;
                units[i].skip_reason = Some(MipSkipReason::NotPresent);
                continue;
            }

            let mip = maybe_mip.unwrap();
            let (w, h) = (mip.width, mip.height);

            if !is_visible {
                // user disabled -> keep geometry, mark skipped
                units[i].index = i;
                units[i].width = w;
                units[i].height = h;
                units[i].included = false;
                units[i].skip_reason = Some(MipSkipReason::UserDisabled);
                continue;
            }

            // present + visible -> encode full baseline CMYK JPEG
            let img = mip.image.as_ref().unwrap();

            let t0 = Instant::now();
            units[i].index = i;

            // ВАЖНО: метод сам ставит width/height/included/jpeg_full/bytes и время за этот мип
            units[i].encode_to_jpeg(img, ctx.has_alpha, ctx.quality)?; // ошибки уже мапятся на BlpError("third-party-error") внутри метода

            // накопительная метрика
            t_acc_ms += t0.elapsed().as_secs_f64() * 1000.0;
            units[i].encode_ms_acc = t_acc_ms;

            println!("mip{}: encoded full JPEG = {} bytes; t_acc={:.3} ms", i, units[i].jpeg_full_bytes, t_acc_ms);
        }

        // 3) write container (all entries are full JPEGs)
        let report = write_blp_full_jpegs(&ctx, &units);
        println!("== encode_blp: OK ==");
        Ok(report)
    }
}
