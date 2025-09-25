// build/fonts.rs

#[cfg(feature = "ui")]
use std::fs;
#[cfg(feature = "ui")]
use std::path::Path;

// ==========================
// NO-UI: пустая генерация
// ==========================
#[cfg(not(feature = "ui"))]
pub fn run_fonts() -> Result<(), Box<dyn std::error::Error>> {
    use std::{env, fs, path::PathBuf};
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    fs::create_dir_all(&out_dir)?;
    let gens = "/* @generated: do not edit */\n\
                pub fn all_fonts_br() -> &'static [(&'static str, &'static [u8])] { &[] }\n";
    fs::write(out_dir.join("fonts_gen.rs"), gens)?;
    Ok(())
}

// ==========================
// UI: полноценная генерация
// ==========================
#[cfg(feature = "ui")]
pub fn run_fonts() -> Result<(), Box<dyn std::error::Error>> {
    use brotli::CompressorWriter;
    use std::{
        env, fs,
        io::{BufReader, Read, Write},
        path::{Path, PathBuf},
        time::Instant,
    };

    let fonts_root = Path::new("assets/fonts");
    println!("cargo:rerun-if-changed={}", fonts_root.display());

    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    fs::create_dir_all(&out_dir)?;

    let mut items: Vec<(String, String)> = Vec::new();

    if !fonts_root.exists() {
        eprintln!("⚠️ {} не существует — шрифты не вшиты", fonts_root.display());
        generate_fonts_rs(&out_dir, &items)?;
        return Ok(());
    }

    // Параметры Brotli (через ENV при желании)
    let quality: u32 = env::var("BROTLI_Q")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(6);
    let lgwin: u32 = env::var("BROTLI_LGWIN")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(22);

    // Собираем *.ttf | *.otf (рекурсивно)
    let mut files: Vec<PathBuf> = Vec::new();
    let mut stack = vec![fonts_root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for ent in fs::read_dir(&dir)? {
            let p = ent?.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                let ext = p
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.to_ascii_lowercase());
                if matches!(ext.as_deref(), Some("ttf") | Some("otf")) {
                    println!("cargo:rerun-if-changed={}", p.display());
                    files.push(p);
                }
            }
        }
    }
    files.sort();

    // Сжатие/пропуск
    for src_path in files {
        let stem = src_path
            .file_stem()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("Font file {} has no stem", src_path.display())))?
            .to_string_lossy()
            .into_owned();
        let ext = src_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("ttf");
        let out_name = format!("{stem}.{ext}.br");
        let dst = out_dir.join(&out_name);

        // Единственное правило: если .br уже есть — не трогаем
        if !dst.exists() {
            let total = fs::metadata(&src_path)
                .map(|m| m.len())
                .unwrap_or(1)
                .max(1);
            let started = Instant::now();

            let f_in = fs::File::open(&src_path)?;
            let mut r = BufReader::with_capacity(1 << 20, f_in);
            let f_out = fs::File::create(&dst)?;
            let mut w = CompressorWriter::new(f_out, 1 << 20, quality, lgwin);

            let mut buf = vec![0u8; 1 << 20];
            loop {
                let n = r.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                w.write_all(&buf[..n])?;
            }
            w.flush()?;
            drop(w);

            let secs = started.elapsed().as_secs_f32();
            let out_size = fs::metadata(&dst)
                .map(|m| m.len())
                .unwrap_or(0);
            eprintln!(
                "🗜️ {} → {} ({:.1} MiB → {:.1} MiB, ratio {:.2}x) за {:.2}s",
                src_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("?"),
                &out_name,
                total as f64 / (1024.0 * 1024.0),
                out_size as f64 / (1024.0 * 1024.0),
                out_size as f64 / total as f64,
                secs
            );
        }

        items.push((stem, out_name));
    }

    generate_fonts_rs(&out_dir, &items)?;
    Ok(())
}

// ---- общий helper для обеих конфигураций (минимальный) ----
#[cfg(feature = "ui")]
fn generate_fonts_rs(out_dir: &Path, items: &[(String, String)]) -> Result<(), Box<dyn std::error::Error>> {
    let mut gens = String::from("/* @generated: do not edit */\n");
    gens.push_str("pub fn all_fonts_br() -> &'static [(&'static str, &'static [u8])] { &[\n");
    for (stem, out_name) in items {
        gens.push_str(&format!("  (\"{}\", include_bytes!(concat!(env!(\"OUT_DIR\"), \"/{}\"))),\n", stem, out_name));
    }
    gens.push_str("] }\n");
    fs::write(out_dir.join("fonts_gen.rs"), gens)?;
    Ok(())
}
