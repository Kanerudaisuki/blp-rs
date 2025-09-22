// build/icons.rs
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

type BuildResult<T> = Result<T, Box<dyn std::error::Error>>;

pub fn run_icons() -> BuildResult<()> {
    println!("cargo:rerun-if-changed=assets/icon.png");

    let src_icon = Path::new("assets/icon.png");
    if !src_icon.exists() {
        return Ok(());
    }

    let out_dir = Path::new("assets/generated");

    // Наборы размеров — как у тебя
    let win_sizes: &[u32] = &[16, 24, 32, 48, 64, 128, 256];
    let mac_bases: &[u32] = &[16, 32, 64, 128, 256, 512]; // часть имеет @2x
    let lin_sizes: &[u32] = &[16, 32, 48, 64, 128, 256, 512];

    let mut expected_outputs: Vec<PathBuf> = vec![out_dir.join("app.ico"), out_dir.join("AppIcon.icns")];
    expected_outputs.extend(
        lin_sizes
            .iter()
            .map(|s| out_dir.join(format!("icons/hicolor/{}x{}/apps/blp-rs.png", s, s))),
    );

    let src_modified = fs::metadata(src_icon)
        .and_then(|meta| meta.modified())
        .ok();

    let mut needs_generation = src_modified.is_none();
    for path in expected_outputs.iter() {
        if !path.exists() {
            needs_generation = true;
            break;
        }
        if let Some(src_modified) = src_modified {
            let up_to_date = fs::metadata(path)
                .and_then(|meta| meta.modified())
                .map(|modified| modified >= src_modified)
                .unwrap_or(false);
            if !up_to_date {
                needs_generation = true;
                break;
            }
        }
    }

    if !needs_generation {
        return Ok(());
    }

    fs::create_dir_all(out_dir)?;

    // Загружаем исходный PNG
    let data = fs::read(src_icon).map_err(|err| io::Error::new(err.kind(), format!("read {}: {err}", src_icon.display())))?;
    let img = image::load_from_memory(&data)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, format!("decode {}: {err}", src_icon.display())))?
        .to_rgba8();
    let (w, h) = (img.width(), img.height());
    if w != h {
        return Err(io::Error::new(io::ErrorKind::InvalidData, format!("icon.png должен быть квадратным (512×512 или 1024×1024), сейчас {w}x{h}")).into());
    }

    generate_ico(out_dir, &img, win_sizes)?;
    generate_icns(out_dir, &img, mac_bases)?;
    generate_linux_hicolor(out_dir, &img, lin_sizes)?;
    // embed_windows_resources(out_dir); // опционально
    Ok(())
}

pub fn generate_ico(out_dir: &Path, img: &image::RgbaImage, sizes: &[u32]) -> BuildResult<()> {
    use ico::{IconDir, IconImage, ResourceType};
    let mut dir = IconDir::new(ResourceType::Icon);
    for &s in sizes {
        let resized = image::imageops::resize(img, s, s, image::imageops::FilterType::Lanczos3);
        let ii = IconImage::from_rgba_data(s, s, resized.into_raw());
        let entry = ico::IconDirEntry::encode(&ii).map_err(|err| io::Error::new(io::ErrorKind::Other, format!("encode ico {s}px: {err}")))?;
        dir.add_entry(entry);
    }
    let path = out_dir.join("app.ico");
    let mut f = fs::File::create(&path).map_err(|err| io::Error::new(err.kind(), format!("create {}: {err}", path.display())))?;
    dir.write(&mut f)
        .map_err(|err| io::Error::new(io::ErrorKind::Other, format!("write {}: {err}", path.display())))?;
    Ok(())
}

pub fn generate_icns(out_dir: &Path, img: &image::RgbaImage, bases: &[u32]) -> BuildResult<()> {
    use icns::{IconFamily, IconType, Image as IcnsImage, PixelFormat};

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
            let icns_img = IcnsImage::from_data(PixelFormat::RGBA, px, px, raw).map_err(|err| io::Error::new(io::ErrorKind::Other, format!("icns from data {px}px: {err}")))?;
            family
                .add_icon_with_type(&icns_img, kind)
                .map_err(|err| io::Error::new(io::ErrorKind::Other, format!("add icns slice {px}px: {err}")))?;
        }
    }
    let path = out_dir.join("AppIcon.icns");
    let mut f = fs::File::create(&path).map_err(|err| io::Error::new(err.kind(), format!("create {}: {err}", path.display())))?;
    family
        .write(&mut f)
        .map_err(|err| io::Error::new(io::ErrorKind::Other, format!("write {}: {err}", path.display())))?;
    Ok(())
}

pub fn generate_linux_hicolor(out_dir: &Path, img: &image::RgbaImage, sizes: &[u32]) -> BuildResult<()> {
    for &s in sizes {
        let resized = image::imageops::resize(img, s, s, image::imageops::FilterType::Lanczos3);
        let dir = out_dir.join(format!("icons/hicolor/{}x{}/apps", s, s));
        fs::create_dir_all(&dir)?;
        let path = dir.join("blp-rs.png");
        let mut f = fs::File::create(&path).map_err(|err| io::Error::new(err.kind(), format!("create {}: {err}", path.display())))?;
        let mut buf = Vec::new();
        image::DynamicImage::ImageRgba8(resized)
            .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .map_err(|err| io::Error::new(io::ErrorKind::Other, format!("encode {}: {err}", path.display())))?;
        f.write_all(&buf)
            .map_err(|err| io::Error::new(err.kind(), format!("write {}: {err}", path.display())))?;
    }
    Ok(())
}

#[allow(dead_code)]
pub fn embed_windows_resources(_out_dir: &Path) {
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        if let Some(path) = _out_dir.join("app.ico").to_str() {
            res.set_icon(path);
        } else {
            eprintln!("app.ico path is not valid UTF-8; skipping Windows resource embedding");
            return;
        }
        res.set("AppUserModelID", "WarRaft.blp-rs");
        if let Err(err) = res.compile() {
            eprintln!("Failed to embed Windows resources: {err}");
        }
    }
}
