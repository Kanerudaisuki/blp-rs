// build/icons.rs
#[cfg(feature = "ui")]
use std::path::{Path, PathBuf};

#[cfg(not(feature = "ui"))]
pub fn run_icons() -> Result<(), Box<dyn std::error::Error>> {
    Ok(()) // без UI — no-op
}

#[cfg(feature = "ui")]
pub fn run_icons() -> Result<(), Box<dyn std::error::Error>> {
    use std::{env, fs, io, path::PathBuf};

    println!("cargo:rerun-if-changed=assets/icon.png");

    let src_icon = Path::new("assets/icon.png");
    if !src_icon.exists() {
        return Ok(());
    }

    let out_dir = Path::new("assets/generated");
    fs::create_dir_all(out_dir)?;

    // Загружаем PNG один раз
    let data = fs::read(src_icon).map_err(|e| io::Error::new(e.kind(), format!("read {}: {e}", src_icon.display())))?;
    let img = image::load_from_memory(&data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("decode {}: {e}", src_icon.display())))?
        .to_rgba8();

    let (w, h) = (img.width(), img.height());
    if w != h {
        return Err(io::Error::new(io::ErrorKind::InvalidData, format!("icon.png должен быть квадратным, сейчас {w}x{h}")).into());
    }

    match env::var("CARGO_CFG_TARGET_OS")
        .unwrap_or_default()
        .as_str()
    {
        "windows" => {
            let outs = [out_dir.join("app.ico")];
            if needs_regen(src_icon, &outs)? {
                generate_ico(out_dir, &img, &[16, 24, 32, 48, 64, 128, 256])?;
            }
            embed_windows_resources(out_dir); // ← вшиваем иконку в PE
        }
        "macos" => {
            let outs = [out_dir.join("AppIcon.icns")];
            if needs_regen(src_icon, &outs)? {
                generate_icns(out_dir, &img, &[16, 32, 64, 128, 256, 512])?;
            }
        }
        _ => {
            let sizes = &[16, 32, 48, 64, 128, 256, 512];
            let mut outs: Vec<PathBuf> = Vec::with_capacity(sizes.len());
            for &s in sizes {
                outs.push(out_dir.join(format!("icons/hicolor/{}x{}/apps/blp.png", s, s)));
            }
            if needs_regen(src_icon, &outs)? {
                generate_linux_hicolor(out_dir, &img, sizes)?;
            }
        }
    }

    Ok(())
}

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
        let ok = fs::metadata(p)
            .and_then(|m| m.modified())
            .map(|t| t >= src_modified)
            .unwrap_or(false);
        if !ok {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(feature = "ui")]
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

#[cfg(feature = "ui")]
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

#[cfg(feature = "ui")]
fn embed_windows_resources(_out_dir: &Path) {
    // Вшиваем только если реально целимся в Windows
    if std::env::var("CARGO_CFG_TARGET_OS")
        .ok()
        .as_deref()
        != Some("windows")
    {
        return;
    }
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        if let Some(path) = _out_dir.join("app.ico").to_str() {
            res.set_icon(path);
        } else {
            eprintln!("app.ico path is not valid UTF-8; skipping Windows resource embedding");
            return;
        }
        res.set("AppUserModelID", "WarRaft.blp");
        if let Err(err) = res.compile() {
            eprintln!("Failed to embed Windows resources: {err}");
        }
    }
}
