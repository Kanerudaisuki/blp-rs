use num_enum::TryFromPrimitive;

#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd, Eq, TryFromPrimitive)]
#[repr(u32)]
pub enum Version {
    BLP0 = 0x424C5030, // "BLP0"
    #[default]
    BLP1 = 0x424C5031, // "BLP1"
    BLP2 = 0x424C5032, // "BLP2"
}
