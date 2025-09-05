use crate::header::Header;
use crate::image_blp::ImageBlp;
use crate::mipmap::Mipmap;
use crate::util::center_crop_to_pow2::center_crop_to_pow2;
use image::imageops::{FilterType, resize};
use std::error::Error;

impl ImageBlp {
    pub(crate) fn decode_image(buf: &[u8]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        // 1) Декодим в RGBA8
        let img = image::load_from_memory(buf)
            .map_err(|e| format!("raster decode failed: {e}"))?
            .to_rgba8();

        let (w, h) = img.dimensions();
        if w == 0 || h == 0 {
            return Err("raster image has zero width or height".into());
        }

        // 2) Центр-кроп под степени двойки
        let base = center_crop_to_pow2(&img); // <-- твоя функция
        let (mut w, mut h) = base.dimensions();
        let (base_w, base_h) = (w, h); // Сохраняем размеры уровня 0

        // 3) Строим мип-цепочку до 1×1 (не останавливаемся, когда одна сторона == 1)
        let mut chain = Vec::with_capacity(16);
        chain.push(base.clone());

        while (w > 1 || h > 1) && chain.len() < 16 {
            let nw = (w / 2).max(1);
            let nh = (h / 2).max(1);
            let next = resize(chain.last().unwrap(), nw, nh, FilterType::Triangle);
            chain.push(next);
            w = nw;
            h = nh;
        }

        // 4) Упаковываем в Vec<Mipmap>
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
            mipmaps.push(Mipmap::default()); // заполняем пустыми
        }

        // 5) В header пишем именно размеры нулевого уровня
        Ok(ImageBlp { header: Header { width: base_w, height: base_h, ..Default::default() }, mipmaps, holes: 0 })
    }
}
