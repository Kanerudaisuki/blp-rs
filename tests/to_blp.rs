// src/tests/to_blp.rs
// Юнит-тест внутри крейта. Максимум логов + перехват panic в encode.

#[cfg(test)]
pub mod to_blp {
    use blp_rs::encode::blp::ctx::ctx::EncoderCtx;
    use blp_rs::encode::blp::options::EncoderOptions;
    use blp_rs::err::error::BlpError;
    use blp_rs::image_blp::{ImageBlp, MAX_MIPS};
    use std::fs;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::path::{Path, PathBuf};
    use std::process::Command;

    #[test]
    fn to_blp_roundtrip_png_encode_then_parse_and_run_ui() -> Result<(), BlpError> {
        eprintln!("== 🧪 START to_blp_roundtrip test ==");

        // --- вход/выход ---
        let root = Path::new("test-data/to-blp");
        let a_png = root.join("a.png");
        let b_blp = root.join("a.blp");

        eprintln!("Input PNG : {}", a_png.display());
        eprintln!("Output BLP: {}", b_blp.display());

        assert!(a_png.exists(), "Missing input PNG: {} (создай test-data/to-blp/a.png)", a_png.display());

        // --- PNG -> ImageBlp ---
        eprintln!("Step 1: reading PNG...");
        let png_bytes = fs::read(&a_png).unwrap_or_else(|e| panic!("Failed to read {}: {e}", a_png.display()));
        eprintln!("  read {} bytes", png_bytes.len());

        eprintln!("Step 2: constructing ImageBlp via ImageBlp::from_buf(&png)...");
        let img = match catch_unwind(AssertUnwindSafe(|| ImageBlp::from_buf(&png_bytes))) {
            Ok(Ok(i)) => i,
            Ok(Err(e)) => {
                eprintln!("  ❌ ImageBlp::from_buf(png) returned error:");
                eprintln!("     • Display: {e}");
                eprintln!("     • Debug  : {e:#?}");
                panic!("from_buf(png) failed with BlpError");
            }
            Err(p) => {
                print_panic_payload("ImageBlp::from_buf(png)", p);
                panic!("from_buf(png) panicked");
            }
        };

        // Доп. лог исходника через image
        let rgba_probe = image::load_from_memory(&png_bytes)
            .map(|d| d.to_rgba8())
            .unwrap_or_else(|err| panic!("image::load_from_memory failed: {err}"));
        let has_alpha_src = rgba_probe.pixels().any(|p| p[3] != 255);
        eprintln!("  source PNG base: {}x{}, has_alpha(src): {}", rgba_probe.width(), rgba_probe.height(), has_alpha_src);
        eprintln!("  parsed ImageBlp base (from PNG): {}x{}", img.mipmaps[0].width, img.mipmaps[0].height);

        // --- encode ---
        let quality = 100u8;
        eprintln!("Step 3: encoding to BLP (quality={}, all mips visible)...", quality);
        let ctx = match catch_unwind(AssertUnwindSafe(|| {
            img.encode_blp(EncoderOptions {
                quality,
                mip_visible: &[], // отсутствующие считаются true
            })
        })) {
            Ok(Ok(c)) => {
                print_ctx_summary(&c);
                c
            }
            Ok(Err(e)) => {
                eprintln!("  ❌ encode_blp returned error:");
                eprintln!("     • Display: {e}");
                eprintln!("     • Debug  : {e:#?}");
                panic!("encode_blp failed with BlpError");
            }
            Err(p) => {
                print_panic_payload("encode_blp", p);
                panic!("encode_blp panicked");
            }
        };

        // --- быстрые инварианты контекста ---
        // 1) всегда MAX_MIPS юнитов
        assert_eq!(ctx.mips.len(), MAX_MIPS, "ctx.mips must contain MAX_MIPS items");

        // 2) included мипов == ctx.visible_count
        let included_cnt = ctx
            .mips
            .iter()
            .filter(|m| m.included)
            .count();
        assert_eq!(included_cnt, ctx.visible_count, "included mip count {} != ctx.visible_count {}", included_cnt, ctx.visible_count);

        // 3) проверяем, что включённые мипы реально закодированы (jpeg_full_bytes > 0)
        for m in ctx.mips.iter().filter(|m| m.included) {
            assert!(m.jpeg_full_bytes > 0 && !m.jpeg_full.is_empty(), "included mip{} must have non-empty jpeg_full", m.index);
            // базовая консистентность
            assert_eq!(m.jpeg_full_bytes, m.jpeg_full.len(), "mip{}: jpeg_full_bytes must equal jpeg_full.len()", m.index);
        }

        // --- write .blp ---
        eprintln!("Step 4: writing .blp...");
        if let Some(parent) = b_blp.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).unwrap_or_else(|e| panic!("create_dir_all {}: {e}", parent.display()));
            }
        }
        fs::write(&b_blp, &ctx.bytes).unwrap_or_else(|e| panic!("Failed to write {}: {e}", b_blp.display()));
        eprintln!("  wrote {} bytes to {}", ctx.bytes.len(), b_blp.display());
        assert!(b_blp.exists(), "BLP was not created at {}", b_blp.display());

        // --- read back & parse ---
        eprintln!("Step 5: reading back .blp...");
        let blp_bytes = fs::read(&b_blp).unwrap_or_else(|e| panic!("Failed to read {}: {e}", b_blp.display()));
        eprintln!("  read {} bytes", blp_bytes.len());

        eprintln!("Step 6: parsing .blp via ImageBlp::from_buf(&blp)...");
        let parsed = match catch_unwind(AssertUnwindSafe(|| ImageBlp::from_buf(&blp_bytes))) {
            Ok(Ok(p)) => p,
            Ok(Err(e)) => {
                eprintln!("  ❌ ImageBlp::from_buf(blp) returned error:");
                eprintln!("     • Display: {e}");
                eprintln!("     • Debug  : {e:#?}");
                panic!("from_buf(blp) failed with BlpError");
            }
            Err(p) => {
                print_panic_payload("ImageBlp::from_buf(blp)", p);
                panic!("from_buf(blp) panicked");
            }
        };
        eprintln!("  parsed header: {}x{}", parsed.header.width, parsed.header.height);

        // --- sanity checks ---
        eprintln!("Step 7: sanity checks...");
        let (w_dec, h_dec) = (parsed.header.width, parsed.header.height);
        assert_eq!((w_dec, h_dec), (ctx.base_width, ctx.base_height), "decoded base size {}x{} != ctx base {}x{}", w_dec, h_dec, ctx.base_width, ctx.base_height);
        eprintln!("  sanity checks OK");

        // --- cargo build --release ---
        eprintln!("Step 8: building release UI binary: `cargo build --release`...");
        let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let build_out = Command::new("cargo")
            .arg("build")
            .arg("--release")
            .current_dir(&crate_root)
            .output()
            .expect("failed to run `cargo build --release`");
        eprintln!("  cargo build --release: status={:?}", build_out.status.code());
        if !build_out.stdout.is_empty() {
            eprintln!("  --- cargo stdout ---\n{}", String::from_utf8_lossy(&build_out.stdout));
        }
        if !build_out.stderr.is_empty() {
            eprintln!("  --- cargo stderr ---\n{}", String::from_utf8_lossy(&build_out.stderr));
        }
        assert!(build_out.status.success(), "`cargo build --release` failed with status {:?}", build_out.status.code());
        eprintln!("  build finished successfully");

        // --- запуск UI ---
        eprintln!("Step 9: running UI (detached)...");
        #[cfg(windows)]
        let bin_name: &str = "blp_rs.exe";
        #[cfg(not(windows))]
        let bin_name: &str = "blp_rs";

        let mut exe = crate_root.clone();
        exe.push("target/release");
        exe.push(bin_name);

        eprintln!("  exe: {}", exe.display());
        assert!(exe.exists(), "UI binary not found at {} after build", exe.display());

        #[cfg(windows)]
        {
            use std::process::Stdio;
            let exe_s = exe.to_string_lossy().to_string();
            let arg_s = b_blp.to_string_lossy().to_string();
            // Launch detached: start "" "exe" "arg"
            let status = Command::new("cmd")
                .arg("/C")
                .arg("start")
                .arg("")
                .arg(exe_s)
                .arg(arg_s)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .expect("failed to start UI (detached) on Windows");
            eprintln!("  ui start status: {:?}", status.code());
        }

        #[cfg(not(windows))]
        {
            use std::process::Stdio;
            // Use nohup to detach from test process
            let cmd = format!("nohup '{}' '{}' >/dev/null 2>&1 &", exe.display(), b_blp.display());
            let status = Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .expect("failed to start UI (detached) on Unix");
            eprintln!("  ui start status: {:?}", status.code());
        }

        eprintln!("== ✅ FINISHED to_blp_roundtrip test ==");
        Ok(())
    }

    fn print_ctx_summary(ctx: &EncoderCtx) {
        eprintln!("== BLP encode summary ==");
        eprintln!("bytes: {} ({:.6} KiB)", ctx.bytes.len(), ctx.bytes.len() as f64 / 1024.0);
        eprintln!("container base: {}x{}", ctx.base_width, ctx.base_height);
        eprintln!("has_alpha: {}", ctx.has_alpha);
        eprintln!("visible mips: {}", ctx.visible_count);
        eprintln!("common header length: {} bytes", ctx.common_header_len);

        let shared_plan = ctx.jpeg_plan.is_some();
        for m in &ctx.mips {
            if m.included {
                let stored = if shared_plan {
                    m.jpeg_slices
                        .map(|s| s.scan_len)
                        .unwrap_or(m.jpeg_full_bytes)
                } else {
                    m.jpeg_full_bytes
                };
                eprintln!("mip{}: {}x{} full {} bytes ({:.2} KiB), stored {} bytes, encode: {:.3} ms", m.index, m.width, m.height, m.jpeg_full_bytes, m.jpeg_full_bytes as f64 / 1024.0, stored, m.encode_ms_acc);
            } else {
                eprintln!("mip{}: SKIPPED", m.index);
            }
        }

        eprintln!("total slices bytes: {}", ctx.total_slices_bytes);
    }

    fn print_panic_payload(phase: &str, p: Box<dyn std::any::Any + Send>) {
        if let Some(s) = p.downcast_ref::<&'static str>() {
            eprintln!("  ❌ panic in {phase}: {s}");
        } else if let Some(s) = p.downcast_ref::<String>() {
            eprintln!("  ❌ panic in {phase}: {s}");
        } else {
            eprintln!("  ❌ panic in {phase}: <non-string payload>");
        }
    }
}
