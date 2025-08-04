use crate::header::{HEADER_SIZE, Header};
use crate::mipmap::Mipmap;
use crate::texture_type::TextureType;
use byteorder::{LittleEndian, ReadBytesExt};
use std::error::Error;
use std::io::{Cursor, Read};

#[derive(Debug)]
pub struct ImageBlp {
    pub header: Header,
    pub mipmaps: Vec<Mipmap>,
}

impl ImageBlp {
    pub fn from_bytes(buf: &[u8]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let mut cursor = Cursor::new(buf);
        let header = Header::parse(&mut cursor)?;

        if cursor.position() != HEADER_SIZE {
            eprintln!("Warning: unexpected cursor position, got {}", cursor.position());
        }

        let mut mipmaps = (0..16)
            .map(|i| {
                let w = (header.width >> i).max(1);
                let h = (header.height >> i).max(1);
                Mipmap { width: w, height: h, image: None }
            })
            .collect::<Vec<_>>();

        match header.texture_type {
            TextureType::JPEG => Self::fill_jpeg(&mut cursor, &header, buf, &mut mipmaps)?,
            TextureType::DIRECT => Self::fill_direct(&mut cursor, &header, buf, &mut mipmaps)?,
        }

        Ok(Self { header, mipmaps })
    }

    fn fill_jpeg(cursor: &mut Cursor<&[u8]>, header: &Header, buf: &[u8], mipmaps: &mut Vec<Mipmap>) -> Result<(), Box<dyn Error + Send + Sync>> {
        let jpeg_header_size = cursor.read_u32::<LittleEndian>()? as usize;
        let mut jpeg_header_chunk = vec![0u8; jpeg_header_size];
        cursor.read_exact(&mut jpeg_header_chunk)?;

        let mut max_data_end = 0;

        for i in 0..16 {
            let offset = header.mipmap_offsets[i] as usize;
            let length = header.mipmap_lengths[i] as usize;

            if length == 0 || offset + length > buf.len() {
                continue;
            }

            let jpeg_chunk = &buf[offset..offset + length];
            let mipmap = Mipmap::read_jpeg(&jpeg_header_chunk, jpeg_chunk)?;

            // Привязка по размеру
            let mut level = 0;
            let mut w = header.width;
            let mut h = header.height;
            while level < 16 {
                if w.max(1) == mipmap.width && h.max(1) == mipmap.height {
                    break;
                }
                if w == 0 && h == 0 {
                    break;
                }
                w >>= 1;
                h >>= 1;
                level += 1;
            }

            if level < 16 {
                mipmaps[level] = mipmap;
            }

            max_data_end = max_data_end.max(offset + length);
        }

        let read_header_end = cursor.position() as usize;
        let total_used = max_data_end.max(read_header_end);
        if total_used != buf.len() {
            println!("Warning: file size = {}, but used only {} bytes ({} extra or missing)", buf.len(), total_used, buf.len() as isize - total_used as isize);
        }

        Ok(())
    }

    fn fill_direct(cursor: &mut Cursor<&[u8]>, header: &Header, buf: &[u8], mipmaps: &mut Vec<Mipmap>) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut palette = [[0u8; 3]; 256];
        for i in 0..256 {
            let color = cursor.read_u32::<LittleEndian>()?;
            palette[i] = [
                ((color >> 16) & 0xFF) as u8, // R
                ((color >> 8) & 0xFF) as u8,  // G
                (color & 0xFF) as u8,         // B
            ];
        }

        for i in 0..16 {
            let offset = header.mipmap_offsets[i] as usize;
            let length = header.mipmap_lengths[i] as usize;
            let width = (header.width >> i).max(1);
            let height = (header.height >> i).max(1);

            if length == 0 || offset + length > buf.len() {
                continue;
            }

            let mut slice_cursor = Cursor::new(&buf[offset..offset + length]);
            let mipmap = Mipmap::read_direct(&mut slice_cursor, width, height, &palette, header.alpha_bits)?;
            mipmaps[i] = mipmap;
        }

        Ok(())
    }
}
