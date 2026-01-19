// use std::simd::prelude::*;

const ARR: [u64; 4] = [ // the printed bits need to be reversed for some reason
    0b0000000000000000000000000000000000000000000000001111111111000000u64.reverse_bits(),
    0b0111111111111111111111111110000001111111111111111111111111100000u64.reverse_bits(),
    0b0000000000000000000000000000000000000000000000000000000000000000u64.reverse_bits(),
    0b0001000000000000000000000000000000000000000000000000000000000000u64.reverse_bits(),
];

pub const FIRST_PASS: Bitset256 = Bitset256(ARR);

#[derive(Debug, Clone, Copy)]
pub struct Bitset256([u64;4]);

impl Bitset256 {
    #[inline]
    pub fn contains(&self, index: usize) -> bool {
        assert!(index < 256);
        let lane = index / 64;
        let bit_within_lane = index % 64;

        let val = self.0[lane];
        (val >> bit_within_lane) & 1 == 1
    }
    #[inline]
    pub fn get_basic_type(&self, index:usize) -> FirstPassType {
        let lane = index / 64;
        let bit_within_lane = index % 64;

        assert!( lane < 4);
        let val = self.0[lane];
        let bucket_type_shift = lane % 2;
        let a = (val >> bit_within_lane) & 1;
        let a = a << bucket_type_shift;
        assert!(a == 0 || a == 1 || a == 2);
        unsafe { std::mem::transmute(a ) }
        
    }
}

#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FirstPassType {
    Skip = 0,
    Number = 1,
    Letter = 2,
}
