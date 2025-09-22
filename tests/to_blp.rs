// src/tests/to_blp.rs
// Юнит-тест внутри крейта. Максимум логов + перехват panic в encode.

#[cfg(test)]
pub mod to_blp {
    use blp_rs::err::error::BlpError;
    use blp_rs::image_blp::ImageBlp;
    use std::fs;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::path::{Path, PathBuf};

    #[test]
    fn to_blp_roundtrip_png_encode_then_parse_and_run_ui() -> Result<(), BlpError> {
        eprintln!("== 🧪 START to_blp_roundtrip test ==");

        // --- вход/выход ---
        let root = Path::new("/Users/nazarpunk/Downloads/_blp");
        let a_png = root.join("bb.png");
        let b_blp = root.join("bb.blp");

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
            img.encode_blp(
                quality,
                &[], // отсутствующие считаются true
            )
        })) {
            Ok(Ok(c)) => c,
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

        // --- запуск UI через cargo run с фичами ---
        eprintln!("Step 8: running UI via `cargo run --release --bin blp-rs-ui --features \"cli ui\"`...");

        use std::process::{Command, Stdio};

        let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        let mut cmd = Command::new("cargo");
        cmd.arg("run")
            .arg("--release")
            .arg("--bin")
            .arg("blp-rs-ui")
            .arg("--features")
            .arg("cli ui") // пробел внутри строки допустим, cargo сам сплитить не будет
            .arg("--");

        // если blp существует — передаём путь, иначе run-ui сам возьмёт дефолт
        if b_blp.exists() {
            cmd.arg(b_blp.to_string_lossy().to_string());
        }

        cmd.current_dir(&crate_root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to start UI via cargo run");

        eprintln!("  UI started detached via cargo run");

        eprintln!("== ✅ FINISHED to_blp_roundtrip test ==");
        Ok(())
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
