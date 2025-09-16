use crate::err::error::BlpError;
use crate::header::Header;
use crate::image_blp::ImageBlp;
use crate::mipmap::Mipmap;
use byteorder::{LittleEndian, ReadBytesExt};
use image::RgbaImage;
use std::io::Cursor;

impl ImageBlp {
    pub(crate) fn decode_direct(
        cursor: &mut Cursor<&[u8]>, //
        header: &Header,
        slices: Vec<Option<&[u8]>>,
        mipmaps: &mut Vec<Mipmap>,
    ) -> Result<(), BlpError> {
        let mut palette = [[0u8; 3]; 256];
        for i in 0..256 {
            let color = cursor.read_u32::<LittleEndian>()?;
            palette[i] = [((color >> 16) & 0xFF) as u8, ((color >> 8) & 0xFF) as u8, (color & 0xFF) as u8];
        }

        for (i, slice_opt) in slices.into_iter().enumerate() {
            if let Some(slice) = slice_opt {
                let mut slice_cursor = Cursor::new(slice);
                let mipmap = Mipmap::decode_direct(&mut slice_cursor, mipmaps[i].width, mipmaps[i].height, &palette, header.alpha_bits)?;
                mipmaps[i] = mipmap;
            }
        }

        Ok(())
    }
}

impl Mipmap {
    pub fn decode_direct<R: std::io::Read>(
        reader: &mut R, //
        width: u32,
        height: u32,
        palette: &[[u8; 3]; 256],
        alpha_bits: u32,
    ) -> Result<Mipmap, BlpError> {
        let pixel_count = (width * height) as usize;

        let mut indices = vec![0u8; pixel_count];
        reader.read_exact(&mut indices)?;

        // Альфа-канал
        let alpha_bytes = match alpha_bits {
            0 => 0,
            1 => (pixel_count + 7) / 8,
            4 => (pixel_count + 1) / 2,
            8 => pixel_count,
            _ => return Err(BlpError::new("blp.version.invalid").with_arg("msg", "unsupported alpha bits")),
        };
        let mut alpha_raw = vec![0u8; alpha_bytes];
        if alpha_bytes > 0 {
            reader.read_exact(&mut alpha_raw)?;
        }

        // Собираем RGBA
        let mut image = RgbaImage::new(width, height);
        for i in 0..pixel_count {
            let idx = indices[i] as usize;
            let [r, g, b] = palette[idx];
            let a = match alpha_bits {
                0 => 255,
                1 => {
                    let byte = alpha_raw[i / 8];
                    let bit = (byte >> (i % 8)) & 1;
                    if bit == 1 { 255 } else { 0 }
                }
                4 => {
                    let byte = alpha_raw[i / 2];
                    let nibble = if i % 2 == 0 { byte & 0x0F } else { byte >> 4 };
                    (nibble << 4) | nibble // чтобы растянуть до 8 бит
                }
                8 => alpha_raw[i],
                _ => 255,
            };
            image
                .get_pixel_mut((i as u32) % width, (i as u32) / width)
                .0 = [r, g, b, a];
        }

        Ok(Mipmap { width, height, image: Some(image) })
    }
}
