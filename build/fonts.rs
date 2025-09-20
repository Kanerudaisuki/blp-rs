// build/fonts.rs
use brotli::CompressorWriter;
use std::{
    env, fs,
    io::{BufReader, Read, Write},
    path::{Path, PathBuf},
    time::{Instant, SystemTime},
};

/// Точка входа из build.rs
pub fn run_fonts() {
    let fonts_root = Path::new("assets/fonts"); // <-- твои шрифты теперь здесь
    println!("cargo:rerun-if-changed={}", fonts_root.display());

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    fs::create_dir_all(&out_dir).expect("mkdir OUT_DIR");

    let mut items: Vec<(String, String)> = Vec::new();

    if !fonts_root.exists() {
        println!("⚠️ {} не существует — шрифты не вшиты", fonts_root.display());
        generate_fonts_rs(&out_dir, &items);
        return;
    }

    // Настройки через окружение
    let quality: u32 = env::var("BROTLI_Q")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(6);
    let lgwin: u32 = env::var("BROTLI_LGWIN")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(22);

    // Рекурсивный сбор списка файлов .ttf/.otf
    let mut files: Vec<PathBuf> = Vec::new();
    let mut stack = vec![fonts_root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for ent in fs::read_dir(&dir).unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display())) {
            let ent = ent.expect("dir entry");
            let p = ent.path();
            if p.is_dir() {
                stack.push(p);
            } else if is_font_file(&p) {
                println!("cargo:rerun-if-changed={}", p.display());
                files.push(p);
            }
        }
    }

    files.sort();

    for path in files {
        let out_name = compress_to_br_no_progress(&path, &out_dir, quality, lgwin).unwrap_or_else(|e| panic!("compress {:?}: {e}", path));
        let stem = path
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        items.push((stem, out_name));
    }

    generate_fonts_rs(&out_dir, &items);
}

fn is_font_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| matches!(e.to_ascii_lowercase().as_str(), "ttf" | "otf"))
        .unwrap_or(false)
}

fn compress_to_br_no_progress(src_path: &Path, out_dir: &Path, quality: u32, lgwin: u32) -> std::io::Result<String> {
    let stem = src_path
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let ext = src_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("ttf");
    let out_name = format!("{stem}.{ext}.br");
    let dst = out_dir.join(&out_name);

    // Пропуск, если .br свежее исходника
    if let (Ok(src_m), Ok(dst_m_opt)) = (mtime(src_path), mtime_opt(&dst)) {
        if let Some(dst_m) = dst_m_opt {
            if dst_m >= src_m {
                println!("⏭️  {} — пропуск (уже сжато)", src_path.display());
                return Ok(out_name);
            }
        }
    }

    // Если уже существует .br (например, ты положил его вручную) — тоже пропустим
    if dst.exists() {
        println!("⏭️  {} — пропуск (найден готовый {})", src_path.display(), dst.display());
        return Ok(out_name);
    }

    let meta = fs::metadata(src_path)?;
    let total = meta.len().max(1);

    println!("🗜️ Brotli q={quality}, lgwin={lgwin}: {}", src_path.display());
    let started = Instant::now();

    let f_in = fs::File::open(src_path)?;
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

    w.flush()?; // закрыть поток
    drop(w);

    let secs = started.elapsed().as_secs_f32();
    let out_size = fs::metadata(&dst)
        .map(|m| m.len())
        .unwrap_or(0);
    let ratio = out_size as f64 / total as f64;

    println!(
        "✅ {} → {} ({:.1} MiB → {:.1} MiB, ratio {:.2}x) за {:.2}s",
        src_path
            .file_name()
            .unwrap()
            .to_string_lossy(),
        out_name,
        total as f64 / (1024.0 * 1024.0),
        out_size as f64 / (1024.0 * 1024.0),
        ratio,
        secs
    );

    Ok(out_name)
}

fn mtime(p: &Path) -> std::io::Result<SystemTime> {
    Ok(fs::metadata(p)?.modified()?)
}
fn mtime_opt(p: &Path) -> std::io::Result<Option<SystemTime>> {
    Ok(fs::metadata(p)
        .and_then(|m| m.modified())
        .ok())
}

fn generate_fonts_rs(out_dir: &Path, items: &[(String, String)]) {
    let mut gens = String::from("/* @generated: do not edit */\n");
    gens.push_str("pub fn all_fonts_br() -> &'static [(&'static str, &'static [u8])] { &[\n");
    for (stem, out_name) in items {
        gens.push_str(&format!("  (\"{}\", include_bytes!(concat!(env!(\"OUT_DIR\"), \"/{}\"))),\n", stem, out_name));
    }
    gens.push_str("] }\n");
    fs::write(out_dir.join("fonts_gen.rs"), gens).expect("write fonts_gen.rs");
}
