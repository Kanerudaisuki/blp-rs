// build/icons.rs
#[cfg(feature = "ui")]
use std::path::{Path, PathBuf};

// ===== единственная публичная точка входа =====
pub fn run_icons() -> Result<(), Box<dyn std::error::Error>> {
    // без UI — no-op
    #[cfg(not(feature = "ui"))]
    {
        return Ok(());
    }

    // основная логика — только при feature = "ui"
    #[cfg(feature = "ui")]
    {
        use std::{fs, io};

        println!("cargo:rerun-if-changed=assets/icon.png");

        let src_icon = Path::new("assets/icon.png");
        if !src_icon.exists() {
            return Ok(());
        }

        let out_dir = Path::new("assets/generated");
        fs::create_dir_all(out_dir)?;

        // Загружаем PNG один раз → RGBA8
        let data = fs::read(src_icon).map_err(|e| io::Error::new(e.kind(), format!("read {}: {e}", src_icon.display())))?;
        let img = image::load_from_memory(&data)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("decode {}: {e}", src_icon.display())))?
            .to_rgba8();

        let (w, h) = (img.width(), img.height());
        if w != h {
            return Err(io::Error::new(io::ErrorKind::InvalidData, format!("icon.png должен быть квадратным, сейчас {w}x{h}")).into());
        }

        // --- OS-специфика: чистые #[cfg] блоки ---

        // Windows: .ico + вшивка в PE
        #[cfg(target_os = "windows")]
        {
            let outs = [out_dir.join("app.ico")];
            if needs_regen(src_icon, &outs)? {
                generate_ico(out_dir, &img, &[16, 24, 32, 48, 64, 128, 256])?;
            }
            embed_windows_resources(out_dir);
        }

        // macOS: .icns
        #[cfg(target_os = "macos")]
        {
            let outs = [out_dir.join("AppIcon.icns")];
            if needs_regen(src_icon, &outs)? {
                generate_icns(out_dir, &img, &[16, 32, 64, 128, 256, 512])?;
            }
        }

        // Linux/прочие: XDG hicolor
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            let sizes = &[16, 32, 48, 64, 128, 256, 512];
            let mut outs: Vec<PathBuf> = Vec::with_capacity(sizes.len());
            for &s in sizes {
                outs.push(out_dir.join(format!("icons/hicolor/{}x{}/apps/blp.png", s, s)));
            }
            if needs_regen(src_icon, &outs)? {
                generate_linux_hicolor(out_dir, &img, sizes)?;
            }
        }

        Ok(())
    }
}

// ===== ниже — хелперы. Они компилируются только когда нужны =====

#[cfg(feature = "ui")]
fn needs_regen(src_icon: &Path, outputs: &[PathBuf]) -> Result<bool, Box<dyn std::error::Error>> {
    use std::fs;
    let src_modified = fs::metadata(src_icon)
        .and_then(|m| m.modified())
        .ok();
    if src_modified.is_none() {
        return Ok(true);
    }
    let src_modified = src_modified.unwrap();
    for p in outputs {
        let fresh = fs::metadata(p)
            .and_then(|m| m.modified())
            .map(|t| t >= src_modified)
            .unwrap_or(false);
        if !fresh {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(all(feature = "ui", target_os = "windows"))]
fn generate_ico(out_dir: &Path, img: &image::RgbaImage, sizes: &[u32]) -> Result<(), Box<dyn std::error::Error>> {
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

#[cfg(feature = "ui")]
fn generate_icns(out_dir: &Path, img: &image::RgbaImage, bases: &[u32]) -> Result<(), Box<dyn std::error::Error>> {
    use icns::{IconFamily, IconType, Image as IcnsImage, PixelFormat};
    use std::{fs, io};

    fn icns_types_for(base: u32) -> &'static [IconType] {
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
    fn target_px(kind: IconType) -> Option<u32> {
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

    let mut family = IconFamily::new();
    for &base in bases {
        for &kind in icns_types_for(base) {
            let Some(px) = target_px(kind) else { continue };
            let resized = image::imageops::resize(img, px, px, image::imageops::FilterType::Lanczos3);
            let raw = resized.into_raw();
            let icns_img = IcnsImage::from_data(PixelFormat::RGBA, px, px, raw).map_err(|e| io::Error::new(io::ErrorKind::Other, format!("icns {px}px: {e}")))?;
            family
                .add_icon_with_type(&icns_img, kind)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("add slice {px}px: {e}")))?;
        }
    }
    let path = out_dir.join("AppIcon.icns");
    let mut f = fs::File::create(&path).map_err(|e| io::Error::new(e.kind(), format!("create {}: {e}", path.display())))?;
    family
        .write(&mut f)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("write {}", e)))?;
    Ok(())
}

#[cfg(all(feature = "ui", not(target_os = "windows"), not(target_os = "macos")))]
fn generate_linux_hicolor(out_dir: &Path, img: &image::RgbaImage, sizes: &[u32]) -> Result<(), Box<dyn std::error::Error>> {
    use std::{fs, io};
    for &s in sizes {
        let resized = image::imageops::resize(img, s, s, image::imageops::FilterType::Lanczos3);
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

// только под Windows реально вшиваем ресурс в exe
#[cfg(all(feature = "ui", target_os = "windows"))]
fn embed_windows_resources(out_dir: &Path) {
    use chrono::Datelike;
    use std::{env, io};

    // ---------- helper: версия -> N.N.N.N ----------
    fn to_winver(v: &str) -> String {
        // допускаем "1.2.3", "1.2.3-beta", "2025.09"
        let mut parts: Vec<u16> = Vec::with_capacity(4);
        for p in v.split('.') {
            let num: u16 = p
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse()
                .unwrap_or(0);
            parts.push(num);
            if parts.len() == 4 {
                break;
            }
        }
        while parts.len() < 4 {
            parts.push(0);
        }
        format!("{}.{}.{}.{}", parts[0], parts[1], parts[2], parts[3])
    }

    // ---------- собрать пути/переменные ----------
    let ico_path = out_dir.join("app.ico");
    if !ico_path.exists() {
        eprintln!("cargo:warning=embed_windows_resources: '{}' not found; ICON resource will be missing", ico_path.display());
    }

    let pkg_name = env::var("CARGO_PKG_NAME").unwrap_or_else(|_| "app".to_string());
    let pkg_desc = env::var("CARGO_PKG_DESCRIPTION").unwrap_or_default();
    let pkg_version_raw = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".to_string());
    let bin_name = env::var("CARGO_BIN_NAME").unwrap_or_else(|_| pkg_name.clone());

    let file_ver = to_winver(&pkg_version_raw);
    if file_ver.ends_with(".0.0") && pkg_version_raw.matches('.').count() < 3 {
        eprintln!("cargo:warning=embed_windows_resources: normalized Product/FileVersion '{}' -> '{}'", pkg_version_raw, file_ver);
    }

    // Авторские права: из AUTHORS, если есть
    let legal = match env::var("CARGO_PKG_AUTHORS") {
        Ok(a) if !a.trim().is_empty() => format!("© {} {}", chrono::Utc::now().year(), a),
        _ => format!("© {} {}", chrono::Utc::now().year(), "Author"),
    };

    // ---------- собрать ресурс ----------
    let mut res = winresource::WindowsResource::new();

    // ИКОНКА
    // ИКОНКА
    if let Some(p) = ico_path.to_str() {
        res.set_icon(p); // set_icon возвращает &mut WindowsResource, но мы его никуда не возвращаем
    } else {
        eprintln!("cargo:warning=embed_windows_resources: app.ico path is not valid UTF-8; skipping icon");
    }

    // VERSIONINFO — эти два поля обязательны, иначе версия не создастся
    res.set("FileVersion", &file_ver);
    res.set("ProductVersion", &file_ver);

    // Строковая таблица
    if !pkg_desc.is_empty() {
        res.set("FileDescription", &pkg_desc);
    }
    res.set("ProductName", &pkg_name);
    res.set("InternalName", &bin_name);
    res.set("OriginalFilename", &format!("{}.exe", bin_name));
    res.set("CompanyName", ""); // если нужно — подставь свою орг-строку
    res.set("LegalCopyright", &legal);

    // Можно явно выставить язык строк (0x0419=ru-RU, 0x0409=en-US)
    // res.set_language(0x0419);

    // ---------- компиляция ----------
    match res.compile() {
        Ok(_) => {
            // полезно видеть, что именно мы вшили
            println!("cargo:warning=ICON embedded: {}", ico_path.display());
            println!("cargo:warning=VERSIONINFO ProductName='{}' FileVersion='{}'", pkg_name, file_ver);
        }
        Err(err) => {
            // подробный разбор частых причин
            eprintln!("cargo:warning=embed_windows_resources: failed to compile resources: {err}");
            if err.kind() == io::ErrorKind::NotFound {
                eprintln!("cargo:warning=Hint: On Windows/MSVC, ensure 'rc.exe' is available (Visual Studio Build Tools).");
                eprintln!("cargo:warning=Hint: On cross-compile setups, install 'llvm-rc' or 'windres' and put it in PATH.");
            }
            // не паникуем: пусть билд идёт без ресурса
        }
    }
}
