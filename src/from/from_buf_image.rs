use crate::image_blp::{Header, ImageBlp, MAX_MIPS};
use crate::mipmap::Mipmap;
use crate::util::center_crop_to_pow2::center_crop_to_pow2;
use image::{
    self,
    imageops::{FilterType, resize},
};
use crate::err::app_err::AppErr;

impl ImageBlp {
    pub(crate) fn from_buf_image(buf: &[u8]) -> Result<Self, AppErr> {
        let guessed: Option<String> = image::guess_format(buf)
            .ok()
            .map(|f| format!("{:?}", f));

        let size = buf.len();

        let img = match image::load_from_memory(buf) {
            Ok(i) => i.to_rgba8(),
            Err(e) => {
                //return Err(err_wire_cause!(ErrKind::DecodeImage { guessed: guessed.clone(), size }, &e));
                return Err(AppErr::new("test"));
            }
        };

        let (w0, h0) = img.dimensions();
        if w0 == 0 || h0 == 0 {
            //return Err(err_wire!(ErrKind::DecodeImage { guessed: guessed.clone(), size }));
            return Err(AppErr::new("test"));
        }

        let base = center_crop_to_pow2(&img);
        let (mut w, mut h) = base.dimensions();
        let (base_w, base_h) = (w, h);

        let mut chain = Vec::with_capacity(16);
        chain.push(base.clone());
        while (w > 1 || h > 1) && chain.len() < MAX_MIPS {
            let nw = (w / 2).max(1);
            let nh = (h / 2).max(1);
            let next = resize(chain.last().unwrap(), nw, nh, FilterType::Triangle);
            chain.push(next);
            w = nw;
            h = nh;
        }

        let mut mipmaps: Vec<Mipmap> = chain
            .into_iter()
            .map(|im| {
                let (w, h) = im.dimensions();
                Mipmap { width: w, height: h, image: Some(im) }
            })
            .collect();

        if mipmaps.len() > MAX_MIPS {
            mipmaps.truncate(MAX_MIPS);
        }
        while mipmaps.len() < MAX_MIPS {
            mipmaps.push(Mipmap::default());
        }

        Ok(ImageBlp { header: Header { width: base_w, height: base_h, ..Default::default() }, mipmaps, holes: 0 })
    }
}
