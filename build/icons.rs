// build/icons.rs
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub fn run_icons() {
    println!("cargo:rerun-if-changed=assets/icon.png");

    let src_icon = Path::new("assets/icon.png");
    if !src_icon.exists() {
        return;
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
        return;
    }

    fs::create_dir_all(out_dir).expect("mkdir assets/generated");

    // Загружаем исходный PNG
    let img = image::load_from_memory(&fs::read(src_icon).expect("read icon.png"))
        .expect("decode icon.png")
        .to_rgba8();
    let (w, h) = (img.width(), img.height());
    assert_eq!(w, h, "icon.png должен быть квадратным (512×512 или 1024×1024)");

    generate_ico(out_dir, &img, win_sizes);
    generate_icns(out_dir, &img, mac_bases);
    generate_linux_hicolor(out_dir, &img, lin_sizes);
    // embed_windows_resources(out_dir); // опционально
}

pub fn generate_ico(out_dir: &Path, img: &image::RgbaImage, sizes: &[u32]) {
    use ico::{IconDir, IconImage, ResourceType};
    let mut dir = IconDir::new(ResourceType::Icon);
    for &s in sizes {
        let resized = image::imageops::resize(img, s, s, image::imageops::FilterType::Lanczos3);
        let ii = IconImage::from_rgba_data(s, s, resized.into_raw());
        dir.add_entry(ico::IconDirEntry::encode(&ii).expect("encode ico"));
    }
    let mut f = fs::File::create(out_dir.join("app.ico")).expect("create app.ico");
    dir.write(&mut f)
        .expect("write app.ico");
}

pub fn generate_icns(out_dir: &Path, img: &image::RgbaImage, bases: &[u32]) {
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
            let icns_img = IcnsImage::from_data(PixelFormat::RGBA, px, px, raw).expect("icns Image::from_data");
            family
                .add_icon_with_type(&icns_img, kind)
                .expect("add icns slice");
        }
    }
    let mut f = fs::File::create(out_dir.join("AppIcon.icns")).expect("create AppIcon.icns");
    family
        .write(&mut f)
        .expect("write icns");
}

pub fn generate_linux_hicolor(out_dir: &Path, img: &image::RgbaImage, sizes: &[u32]) {
    for &s in sizes {
        let resized = image::imageops::resize(img, s, s, image::imageops::FilterType::Lanczos3);
        let dir = out_dir.join(format!("icons/hicolor/{}x{}/apps", s, s));
        fs::create_dir_all(&dir).expect("mkdir -p hicolor/apps");
        let path = dir.join("blp-rs.png");
        let mut f = fs::File::create(&path).expect("create hicolor png");
        let mut buf = Vec::new();
        image::DynamicImage::ImageRgba8(resized)
            .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .expect("encode linux png");
        f.write_all(&buf)
            .expect("write linux png");
    }
}

#[allow(dead_code)]
pub fn embed_windows_resources(_out_dir: &Path) {
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon(
            _out_dir
                .join("app.ico")
                .to_str()
                .unwrap(),
        );
        res.set("AppUserModelID", "WarRaft.blp-rs");
        let _ = res.compile();
    }
}
