#[cfg(test)]
mod scan {
    use blp_rs::header::{HEADER_SIZE, Header};
    use blp_rs::image_blp::ImageBlp;
    use image::{DynamicImage, ImageFormat};
    use std::collections::HashMap;
    use std::fs::{self, File};
    use std::io::{BufWriter, Cursor, Read};
    use std::path::{Path, PathBuf};
    use walkdir::WalkDir;

    const OUT_DIR: &str = "test-data/scan";

    #[test]
    fn scan() {
        let dest = Path::new("/Users/nazarpunk/IdeaProjects/War3.mpq/extract");
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
            if fs::File::open(path)
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
}
