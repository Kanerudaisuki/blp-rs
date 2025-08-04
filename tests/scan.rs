#[cfg(test)]
mod scan {
    use blp_rs::header::{HEADER_SIZE, Header};
    use blp_rs::image_blp::ImageBlp;
    use blp_rs::texture_type::TextureType;
    use image::{DynamicImage, ImageFormat};
    use std::collections::HashMap;
    use std::fs::{self, File};
    use std::io::{BufWriter, Cursor, Read};
    use std::path::{Path, PathBuf};
    use walkdir::WalkDir;

    const DEST_DIR: &str = "/Users/nazarpunk/IdeaProjects/War3.mpq/extract";
    const OUT_DIR: &str = "test-data/scan";

    #[test]
    fn scan() {
        let dest = Path::new(DEST_DIR);
        let out = Path::new(OUT_DIR);

        if out.exists() {
            match fs::remove_dir_all(&out) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("⚠️ Не удалось удалить папку {}: {e}", out.display());
                    // Можно упасть, если это критично:
                    // panic!("Can't delete {}", out.display());
                }
            }
        }

        fs::create_dir_all(out).unwrap();

        let mut seen: HashMap<String, PathBuf> = HashMap::new();

        for entry in WalkDir::new(dest)
            .into_iter()
            .filter_map(Result::ok)
        {
            let path = entry.path();
            if !path.is_file()
                || path
                    .extension()
                    .map(|e| e != "blp")
                    .unwrap_or(true)
            {
                continue;
            }

            let mut buf = vec![0u8; HEADER_SIZE as usize];
            if File::open(path)
                .and_then(|mut f| f.read_exact(&mut buf))
                .is_err()
            {
                continue;
            }

            let mut cursor = Cursor::new(&buf[..]);
            let header = match Header::parse(&mut cursor) {
                Ok(h) => h,
                Err(_) => continue,
            };

            let name = format!("{:?}_tt{}_c{}_ab{}_at{}_m{}_{}x{}.blp", header.version, header.texture_type as u8, header.compression, header.alpha_bits, header.alpha_type, header.has_mips, header.width, header.height,);

            if seen.contains_key(&name) {
                continue;
            }

            let dst = Path::new(OUT_DIR).join(&name);
            if let Err(e) = fs::copy(path, &dst) {
                eprintln!("Failed to copy {:?} → {:?}: {e}", path, dst);
                continue;
            }

            seen.insert(name, path.to_path_buf());
        }

        println!("Done: {} unique textures copied.", seen.len());
    }

    #[test]
    fn convert() {
        let dir = Path::new(OUT_DIR);

        for entry in WalkDir::new(dir)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "blp")
                    .unwrap_or(false)
            })
        {
            let path = entry.path();
            let buf = match fs::read(path) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("Failed to read {}: {e}", path.display());
                    continue;
                }
            };

            let image = match ImageBlp::from_bytes(&buf) {
                Ok(img) => img,
                Err(e) => {
                    eprintln!("Failed to parse {}: {e}", path.display());
                    continue;
                }
            };

            let stem = path
                .file_stem()
                .unwrap()
                .to_string_lossy();

            for mip in &image.mipmaps {
                if let Some(rgba) = &mip.image {
                    let filename = format!("{stem}_{}x{}.png", mip.width, mip.height);
                    let output_path = path.with_file_name(filename);

                    match File::create(&output_path) {
                        Ok(file) => {
                            let mut writer = BufWriter::new(file);
                            if let Err(e) = DynamicImage::ImageRgba8(rgba.clone()).write_to(&mut writer, ImageFormat::Png) {
                                eprintln!("Failed to write PNG {}: {e}", output_path.display());
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to create file {}: {e}", output_path.display());
                        }
                    }
                }
            }
        }

        println!("Conversion done.");
    }

    #[test]
    fn all() {
        use std::collections::BTreeMap;
        use std::fs;
        use std::path::Path;
        use std::time::Instant;
        use walkdir::WalkDir;

        fn fmt_bytes(bytes: usize) -> String {
            const UNITS: [&str; 5] = ["bytes", "KiB", "MiB", "GiB", "TiB"];
            let mut size = bytes as f64;
            let mut unit = 0;
            while size >= 1024.0 && unit < UNITS.len() - 1 {
                size /= 1024.0;
                unit += 1;
            }
            format!("{:.2} {} ({} bytes)", size, UNITS[unit], bytes)
        }

        fn print_resolution_stats(title: &str, map: &BTreeMap<(u32, u32), (usize, f64, usize)>) {
            println!("\n🔹 {}", title);
            println!("   {:>8}   {:>6}   {:>9}   {:>9}   {:>9}   {:>9}", "Res", "Count", "Avg ms", "Total s", "MP/sec", "MiB/sec");

            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_by_key(|&(res, _)| res.0 * res.1); // sort by area

            for ((w, h), &(count, total_time, total_bytes)) in entries {
                let avg_ms = total_time * 1000.0 / count as f64;
                let mp = (*w as f64 * *h as f64) / 1_000_000.0;
                let total_mp = mp * count as f64;
                let mp_per_sec = total_mp / total_time.max(0.0001);

                let mib = total_bytes as f64 / (1024.0 * 1024.0);
                let mib_per_sec = mib / total_time.max(0.0001);

                println!("   {:>4}×{:<4}   {:>6}   {:>9.3}   {:>9.3}   {:>9.2}   {:>9.2}", w, h, count, avg_ms, total_time, mp_per_sec, mib_per_sec);
            }
        }

        type Stats = (usize, f64, usize); // count, total_time_sec, total_bytes

        let mut total = 0;
        let mut failed = 0;
        let mut jpeg_total = 0;
        let mut jpeg_total_size = 0;
        let mut jpeg_holes = 0;
        let mut jpeg_with_holes = 0;
        let mut direct_total = 0;
        let mut direct_total_size = 0;
        let mut direct_holes = 0;
        let mut direct_with_holes = 0;

        let mut jpeg_by_res: BTreeMap<(u32, u32), Stats> = BTreeMap::new();
        let mut direct_by_res: BTreeMap<(u32, u32), Stats> = BTreeMap::new();

        let dir = Path::new(DEST_DIR);
        assert!(dir.exists(), "Directory does not exist: {}", DEST_DIR);

        let start = Instant::now();

        for entry in WalkDir::new(dir)
            .into_iter()
            .filter_map(Result::ok)
        {
            let path = entry.path();
            if path
                .extension()
                .map(|ext| ext.eq_ignore_ascii_case("blp"))
                != Some(true)
            {
                continue;
            }

            total += 1;

            let data = match fs::read(path) {
                Ok(data) => data,
                Err(e) => {
                    eprintln!("❌ Failed to read {:?}: {}", path, e);
                    failed += 1;
                    continue;
                }
            };

            let decode_start = Instant::now();
            let result = ImageBlp::from_bytes(&data);
            let decode_time = decode_start.elapsed().as_secs_f64();

            match result {
                Ok(blp) => {
                    let res = (blp.header.width, blp.header.height);
                    let size = data.len();

                    match blp.header.texture_type {
                        TextureType::JPEG => {
                            jpeg_total += 1;
                            jpeg_total_size += size;
                            jpeg_holes += blp.holes;
                            if blp.holes > 0 {
                                jpeg_with_holes += 1;
                            }

                            let entry = jpeg_by_res
                                .entry(res)
                                .or_insert((0, 0.0, 0));
                            entry.0 += 1;
                            entry.1 += decode_time;
                            entry.2 += size;
                        }
                        TextureType::DIRECT => {
                            direct_total += 1;
                            direct_total_size += size;
                            direct_holes += blp.holes;
                            if blp.holes > 0 {
                                direct_with_holes += 1;
                            }

                            let entry = direct_by_res
                                .entry(res)
                                .or_insert((0, 0.0, 0));
                            entry.0 += 1;
                            entry.1 += decode_time;
                            entry.2 += size;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to parse {:?}: {}", path, e);
                    failed += 1;
                }
            }
        }

        let total_time = start.elapsed().as_secs_f64();
        let parsed = total - failed;
        let avg_time = total_time / parsed.max(1) as f64;

        println!("\n📦 Total BLP files      : {}", total);
        println!("✅ Parsed successfully  : {}", parsed);
        println!("❌ Failed to parse       : {}", failed);
        println!("⏱  Total time           : {:.3} s", total_time);
        println!("📈 Avg time per file    : {:.3} ms", avg_time * 1000.0);

        println!("\n🔹 JPEG Stats");
        println!("   • Count              : {}", jpeg_total);
        println!("   • Total size         : {}", fmt_bytes(jpeg_total_size));
        println!("   • Holes              : {} ({} files, avg = {} bytes)", jpeg_holes, jpeg_with_holes, if jpeg_with_holes > 0 { jpeg_holes / jpeg_with_holes } else { 0 });

        println!("\n🔹 DIRECT Stats");
        println!("   • Count              : {}", direct_total);
        println!("   • Total size         : {}", fmt_bytes(direct_total_size));
        println!("   • Holes              : {} ({} files, avg = {} bytes)", direct_holes, direct_with_holes, if direct_with_holes > 0 { direct_holes / direct_with_holes } else { 0 });

        print_resolution_stats("JPEG decode performance", &jpeg_by_res);
        print_resolution_stats("DIRECT decode performance", &direct_by_res);
    }
}
