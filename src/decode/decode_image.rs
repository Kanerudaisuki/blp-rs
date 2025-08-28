use crate::header::Header;
use crate::image_blp::ImageBlp;
use crate::mipmap::Mipmap;
use crate::util::center_crop_to_pow2::center_crop_to_pow2;
use image::imageops::{FilterType, resize};
use std::error::Error;

impl ImageBlp {
    pub(crate) fn decode_image(buf: &[u8]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        // 👉 Иначе пробуем открыть через image crate
        let img = image::load_from_memory(buf)
            .map_err(|e| format!("raster decode failed: {e}"))?
            .to_rgba8();

        let (w, h) = img.dimensions();
        if w == 0 || h == 0 {
            return Err("raster image has zero width or height".into());
        }

        // Центр-кропим под степени двойки
        let cropped = center_crop_to_pow2(&img);

        // Делаем mip-цепочку до 1
        let (mut w, mut h) = cropped.dimensions();
        let mut chain = Vec::with_capacity(16);
        chain.push(cropped.clone());
        while (w > 1 && h > 1) && chain.len() < 16 {
            let nw = (w / 2).max(1);
            let nh = (h / 2).max(1);
            let next = resize(chain.last().unwrap(), nw, nh, FilterType::Triangle);
            chain.push(next);
            w = nw;
            h = nh;
            if w == 1 || h == 1 {
                break; // стоп на 2×1 / 1×N или естественный 1×1 (из квадрата)
            }
        }

        let mut mipmaps: Vec<Mipmap> = chain
            .into_iter()
            .map(|im| {
                let (w, h) = im.dimensions();
                Mipmap { width: w, height: h, image: Some(im) }
            })
            .collect();

        if mipmaps.len() > 16 {
            mipmaps.truncate(16);
        }
        while mipmaps.len() < 16 {
            mipmaps.push(Mipmap::default());
        }

        Ok(ImageBlp {
            header: Header {
                width: w, //
                height: h,
                ..Default::default()
            },
            mipmaps,
            holes: 0,
        })
    }
}
