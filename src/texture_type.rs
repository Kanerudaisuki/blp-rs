use num_enum::TryFromPrimitive;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, TryFromPrimitive)]
#[repr(u32)]
pub enum TextureType {
    JPEG = 0,
    DIRECT = 1,
}
