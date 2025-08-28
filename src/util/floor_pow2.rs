#[inline]
pub fn floor_pow2(x: u32) -> u32 {
    1 << (31 - x.max(1).leading_zeros())
}
