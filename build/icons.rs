// no-op без UI
#[cfg(not(feature = "ui"))]
pub fn run_icons() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

// всё остальное — только при ui
#[cfg(feature = "ui")]
mod impls {
    use std::error::Error;
    use std::fs;
    use std::path::Path;

    pub fn run_icons() -> Result<(), Box<dyn Error>> {
        let src_icon = Path::new("assets/icon.png");
        let out_dir = Path::new("assets/generated");
        fs::create_dir_all(out_dir)?;

        #[cfg(target_os = "windows")]
        do_windows(src_icon, out_dir)?;

        #[cfg(target_os = "macos")]
        do_macos(src_icon, out_dir)?;

        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        do_linux(src_icon, out_dir)?;

        Ok(())
    }

    // --- WINDOWS ---
    #[cfg(target_os = "windows")]
    fn do_windows(src_icon: &Path, out_dir: &Path) -> Result<(), Box<dyn Error>> {
        use std::{fs, io};

        let ico_path = out_dir.join("app.ico");
        if !ico_path.exists() && src_icon.exists() {
            let data = fs::read(src_icon).map_err(|e| io::Error::new(e.kind(), format!("read {}: {e}", src_icon.display())))?;
            let img = image::load_from_memory(&data)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("decode {}: {e}", src_icon.display())))?
                .to_rgba8();
            let (w, h) = (img.width(), img.height());
            if w != h {
                return Err(io::Error::new(io::ErrorKind::InvalidData, format!("icon.png должен быть квадратным, сейчас {w}x{h}")).into());
            }
            generate_ico(out_dir, &img, &[16, 24, 32, 48, 64, 128, 256])?;
        }
        embed_windows_resources(out_dir);
        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn generate_ico(out_dir: &Path, img: &image::RgbaImage, sizes: &[u32]) -> Result<(), Box<dyn Error>> {
        use ico::{IconDir, IconImage, ResourceType};
        use std::{fs, io};
        let mut dir = IconDir::new(ResourceType::Icon);
        for &s in sizes {
            let resized = image::imageops::resize(img, s, s, image::imageops::FilterType::Lanczos3);
            let ii = IconImage::from_rgba_data(s, s, resized.into_raw());
            let entry = ico::IconDirEntry::encode(&ii).map_err(|e| io::Error::new(io::ErrorKind::Other, format!("encode ico {s}px: {e}")))?;
            dir.add_entry(entry);
        }
        let path = out_dir.join("app.ico");
        let mut f = fs::File::create(&path).map_err(|e| io::Error::new(e.kind(), format!("create {}: {e}", path.display())))?;
        dir.write(&mut f)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("write {}: {e}", path.display())))?;
        Ok(())
    }

    #[cfg(all(feature = "ui", target_os = "windows"))]
    #[cfg(all(feature = "ui", target_os = "windows"))]
    fn embed_windows_resources(out_dir: &Path) {
        use std::{env, io, path::Path};

        // нормализуем версию в N.N.N.N
        fn normalize_version(v: &str) -> String {
            let mut parts = [0u16; 4];
            let mut i = 0usize;
            for seg in v.split('.') {
                if i >= 4 {
                    break;
                }
                let num = seg
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>();
                parts[i] = num.parse::<u16>().unwrap_or(0);
                i += 1;
            }
            format!("{}.{}.{}.{}", parts[0], parts[1], parts[2], parts[3])
        }

        // получаем значения из окружения (Cargo подставит)
        let pkg_name = env::var("CARGO_PKG_NAME").unwrap_or_else(|_| "app".to_string());
        let pkg_desc = env::var("CARGO_PKG_DESCRIPTION").unwrap_or_else(|_| pkg_name.clone());
        let pkg_version_raw = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".to_string());
        let pkg_authors = env::var("CARGO_PKG_AUTHORS").unwrap_or_default();
        let bin_name = env::var("CARGO_BIN_NAME").unwrap_or_else(|_| pkg_name.clone());

        let file_version = normalize_version(&pkg_version_raw);

        // Company: первая часть AUTHORS или fallback
        let company = pkg_authors
            .split(':')
            .next()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .unwrap_or("Unknown Company")
            .to_string();

        let copyright = if !pkg_authors.is_empty() { format!("© {}", pkg_authors) } else { format!("© {}", company) };

        // Иконка
        let ico_path = out_dir.join("app.ico");
        let mut res = winresource::WindowsResource::new();

        match ico_path.to_str() {
            Some(p) => {
                if Path::new(p).exists() {
                    res.set_icon(p);
                    println!("cargo:warning=winresource: will embed ICON: {}", ico_path.display());
                } else {
                    println!("cargo:warning=winresource: app.ico {} not found — embedding without ICON", ico_path.display());
                }
            }
            None => {
                eprintln!("cargo:warning=winresource: app.ico path is not valid UTF-8; skipping icon embedding");
            }
        }

        // VERSIONINFO числовые поля
        res.set("FileVersion", &file_version);
        res.set("ProductVersion", &file_version);

        // Строковая таблица (Explorer читает эти поля)
        res.set("FileDescription", &pkg_desc);
        res.set("ProductName", &pkg_name);
        res.set("CompanyName", &company);
        res.set("InternalName", &bin_name);
        res.set("OriginalFilename", &format!("{}.exe", bin_name));
        res.set("LegalCopyright", &copyright);

        // Язык: ru-RU (0x0419). Поменяй на 0x0409 если нужно en-US
        res.set_language(0x0419);

        // Компиляция ресурсов — логируем подробно
        match res.compile() {
            Ok(_) => {
                println!("cargo:warning=winresource: resources compiled. Product='{}' FileVersion='{}' Lang=0x0419", pkg_name, file_version);
            }
            Err(err) => {
                eprintln!("cargo:warning=winresource: failed to compile resources: {}", err);
                if err.kind() == io::ErrorKind::NotFound {
                    eprintln!("cargo:warning=winresource: rc tool not found. On MSVC ensure 'rc.exe' (VS Build Tools) is in PATH; on Unix install 'llvm-rc' or 'windres'.");
                }
            }
        }
    }

    // --- macOS ---
    #[cfg(target_os = "macos")]
    fn do_macos(src_icon: &Path, out_dir: &Path) -> Result<(), Box<dyn Error>> {
        use icns::{IconFamily, IconType, Image as IcnsImage, PixelFormat};
        use std::{fs, io};

        let icns_path = out_dir.join("AppIcon.icns");
        if icns_path.exists() || !src_icon.exists() {
            return Ok(());
        }

        let data = fs::read(src_icon).map_err(|e| io::Error::new(e.kind(), format!("read {}: {e}", src_icon.display())))?;
        let img = image::load_from_memory(&data)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("decode {}: {e}", src_icon.display())))?
            .to_rgba8();

        let mut family = IconFamily::new();
        fn kinds(base: u32) -> &'static [IconType] {
            match base {
                16 => &[IconType::RGBA32_16x16, IconType::RGBA32_16x16_2x],
                32 => &[IconType::RGBA32_32x32, IconType::RGBA32_32x32_2x],
                64 => &[IconType::RGBA32_64x64],
                128 => &[IconType::RGBA32_128x128, IconType::RGBA32_128x128_2x],
                256 => &[IconType::RGBA32_256x256, IconType::RGBA32_256x256_2x],
                512 => &[IconType::RGBA32_512x512, IconType::RGBA32_512x512_2x],
                _ => &[],
            }
        }
        fn px(kind: IconType) -> Option<u32> {
            Some(match kind {
                IconType::RGBA32_16x16 => 16,
                IconType::RGBA32_16x16_2x => 32,
                IconType::RGBA32_32x32 => 32,
                IconType::RGBA32_32x32_2x => 64,
                IconType::RGBA32_64x64 => 64,
                IconType::RGBA32_128x128 => 128,
                IconType::RGBA32_128x128_2x => 256,
                IconType::RGBA32_256x256 => 256,
                IconType::RGBA32_256x256_2x => 512,
                IconType::RGBA32_512x512 => 512,
                IconType::RGBA32_512x512_2x => 1024,
                _ => return None,
            })
        }

        for &base in &[16, 32, 64, 128, 256, 512] {
            for &kind in kinds(base) {
                if let Some(s) = px(kind) {
                    let resized = image::imageops::resize(&img, s, s, image::imageops::FilterType::Lanczos3);
                    let raw = resized.into_raw();
                    let icns_img = IcnsImage::from_data(PixelFormat::RGBA, s, s, raw).map_err(|e| io::Error::new(io::ErrorKind::Other, format!("icns {s}px: {e}")))?;
                    family
                        .add_icon_with_type(&icns_img, kind)
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("add slice {s}px: {e}")))?;
                }
            }
        }

        let mut f = fs::File::create(&icns_path).map_err(|e| io::Error::new(e.kind(), format!("create {}: {e}", icns_path.display())))?;
        family
            .write(&mut f)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("write {}: {e}", icns_path.display())))?;
        Ok(())
    }

    // --- Linux/прочее ---
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    fn do_linux(src_icon: &Path, out_dir: &Path) -> Result<(), Box<dyn Error>> {
        use std::path::PathBuf;
        use std::{fs, io};

        let sizes = &[16, 32, 48, 64, 128, 256, 512];
        let targets: Vec<PathBuf> = sizes
            .iter()
            .map(|s| out_dir.join(format!("icons/hicolor/{}x{}/apps/blp.png", s, s)))
            .collect();

        if targets.iter().all(|p| p.exists()) || !src_icon.exists() {
            return Ok(());
        }

        let data = fs::read(src_icon).map_err(|e| io::Error::new(e.kind(), format!("read {}: {e}", src_icon.display())))?;
        let img = image::load_from_memory(&data)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("decode {}: {e}", src_icon.display())))?
            .to_rgba8();

        for &s in sizes {
            let resized = image::imageops::resize(&img, s, s, image::imageops::FilterType::Lanczos3);
            let dir = out_dir.join(format!("icons/hicolor/{}x{}/apps", s, s));
            fs::create_dir_all(&dir)?;
            let path = dir.join("blp.png");

            let mut buf = Vec::new();
            image::DynamicImage::ImageRgba8(resized)
                .write_to(&mut io::Cursor::new(&mut buf), image::ImageFormat::Png)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("encode {}: {e}", path.display())))?;

            let mut f = fs::File::create(&path).map_err(|e| io::Error::new(e.kind(), format!("create {}: {e}", path.display())))?;
            use std::io::Write;
            f.write_all(&buf)
                .map_err(|e| io::Error::new(e.kind(), format!("write {}: {e}", path.display())))?;
        }

        Ok(())
    }
}

// re-export
#[cfg(feature = "ui")]
pub use impls::run_icons;
