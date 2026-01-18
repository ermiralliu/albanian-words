use std::simd::prelude::*;

const ARR: [u64; 4] = [ // the printed bits need to be reversed for some reason
    0b0000000000000000000000000000000000000000000000001111111111000000u64.reverse_bits(),
    0b0111111111111111111111111110000001111111111111111111111111100000u64.reverse_bits(),
    0b0000000000000000000000000000000000000000000000000000000000000000u64.reverse_bits(),
    0b0001000000000000000000000000000000000000000000000000000000000000u64.reverse_bits(),
];

pub const BITSET: Bitset256 = Bitset256(u64x4::from_array(ARR));

#[derive(Debug, Clone, Copy)]
pub struct Bitset256(u64x4);

impl Bitset256 {
    #[inline]
    pub fn contains(&self, index: usize) -> bool {
        assert!(index < 256);
        let lane = index / 64;
        let bit_within_lane = index % 64;

        let val = self.0[lane];
        (val >> bit_within_lane) & 1 == 1
    }
}
