// build.rs
use std::{fs, io::Write, path::Path};

fn main() {
    // Пересобирать при изменении исходника
    println!("cargo:rerun-if-changed=assets/icon.png");

    let src = Path::new("assets/icon.png");
    if !src.exists() {
        eprintln!("⚠️  assets/icon.png не найден — пропускаю генерацию иконок");
        return;
    }

    // Куда складываем артефакты
    let out_dir = Path::new("assets/generated");
    fs::create_dir_all(out_dir).expect("mkdir assets/generated");

    // Загружаем исходный PNG
    let img = image::load_from_memory(&fs::read(src).expect("read icon.png"))
        .expect("decode icon.png")
        .to_rgba8();
    let (w, h) = (img.width(), img.height());
    assert_eq!(w, h, "icon.png должен быть квадратным (512×512 или 1024×1024)");

    // Наборы размеров
    let win_sizes: &[u32] = &[16, 24, 32, 48, 64, 128, 256];
    let mac_bases: &[u32] = &[16, 32, 64, 128, 256, 512]; // часть имеет @2x
    let lin_sizes: &[u32] = &[16, 32, 48, 64, 128, 256, 512];

    // 1) Windows .ico
    generate_ico(out_dir, &img, win_sizes);

    // 2) macOS .icns (RGBA32-типы + ретина) — БЕЗ from_png, через from_data
    generate_icns(out_dir, &img, mac_bases);

    // 3) Linux hicolor PNG
    generate_linux_hicolor(out_dir, &img, lin_sizes);

    // Если нужно — можно вшить ресурсы в Windows-EXE:
    // embed_windows_resources(out_dir);
}

fn generate_ico(out_dir: &Path, img: &image::RgbaImage, sizes: &[u32]) {
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

fn generate_icns(out_dir: &Path, img: &image::RgbaImage, bases: &[u32]) {
    use icns::{IconFamily, IconType, Image as IcnsImage, PixelFormat};

    // Для каждого базового размера — какие типы кладём
    fn icns_types_for(base: u32) -> &'static [IconType] {
        match base {
            16 => &[IconType::RGBA32_16x16, IconType::RGBA32_16x16_2x],      // 16, 32
            32 => &[IconType::RGBA32_32x32, IconType::RGBA32_32x32_2x],      // 32, 64
            64 => &[IconType::RGBA32_64x64],                                 // 64 (без @2x)
            128 => &[IconType::RGBA32_128x128, IconType::RGBA32_128x128_2x], // 128, 256
            256 => &[IconType::RGBA32_256x256, IconType::RGBA32_256x256_2x], // 256, 512
            512 => &[IconType::RGBA32_512x512, IconType::RGBA32_512x512_2x], // 512, 1024
            _ => &[],
        }
    }

    // Фактический размер пикселей для каждого IconType
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
            let px = match target_px(kind) {
                Some(v) => v,
                None => continue,
            };

            // ресайз → RGBA8 raw
            let resized = image::imageops::resize(img, px, px, image::imageops::FilterType::Lanczos3);
            let raw = resized.into_raw(); // RGBA

            // ✅ создаём icns::Image из сырых RGBA-данных
            let icns_img = IcnsImage::from_data(
                PixelFormat::RGBA, // первым аргументом идёт формат
                px,                // ширина
                px,                // высота
                raw,               // байты
            )
            .expect("icns Image::from_data");
            // кладём как нужный тип
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

fn generate_linux_hicolor(out_dir: &Path, img: &image::RgbaImage, sizes: &[u32]) {
    for &s in sizes {
        let resized = image::imageops::resize(img, s, s, image::imageops::FilterType::Lanczos3);
        let dir = out_dir.join(format!("icons/hicolor/{}x{}/apps", s, s));
        fs::create_dir_all(&dir).expect("mkdir -p hicolor/apps");
        let path = dir.join("blp-rs.png");
        let mut f = fs::File::create(path).expect("create hicolor png");
        let mut buf = Vec::new();
        image::DynamicImage::ImageRgba8(resized)
            .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .expect("encode linux png");
        f.write_all(&buf)
            .expect("write linux png");
    }
}

// Опционально: вшивание .ico и AUMID для Windows EXE (фикс pinned-иконки)
#[allow(dead_code)]
fn embed_windows_resources(_out_dir: &Path) {
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
