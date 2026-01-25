use std::simd::prelude::*;

pub const ARR: [u64; 4] = [
    // the printed bits need to be reversed for some reason
    0b0000000000000000000000000000000000000000000000001111111111000000u64.reverse_bits(),
    0b0111111111111111111111111110000001111111111111111111111111100000u64.reverse_bits(),
    0b0000000000000000000000000000000000000000000000000000000000000000u64.reverse_bits(),
    0b0001000000000000000000000000000000000000000000000000000000000000u64.reverse_bits(),
];

#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FirstPassType {
    Skip = 0,
    Number = 1,
    Letter = 2,
}

#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecondPassType {
    END = 0,
    Letter = 1,
    XC3 = 2,
    XCC = 3,
}

#[inline(always)]
pub fn get_first_pass_type(index: u8, reg: u64x4) -> FirstPassType {
    let index = index as usize;
    // let lane = index / 64;
    // let bit_within_lane = index % 64;
    // Just to make sure at this point *crying emoji*
    let lane = index >> 6;
    let bit_within_lane = index & 63;

    assert!(lane < 4);
    let val = match lane {
        0 => reg[0],
        1 => reg[1],
        2 => reg[2],
        3 => reg[3],
        _ => unreachable!(),
    };

    let bucket_type_shift = lane % 2;
    let a = (val >> bit_within_lane) & 1;
    let a = a << bucket_type_shift;
    assert!(a == 0 || a == 1 || a == 2);
    unsafe { std::mem::transmute(a) }
}

#[inline(always)]
pub fn get_second_pass_type(index: u8, reg: u64x4) -> SecondPassType {
    let index = index as usize;
    // let lane = index / 64;
    // let bit_within_lane = index % 64;
    let lane = index >> 6;
    let bit_within_lane = index & 63;

    // 1. Check for 0xCC: all ones if true, all zeros if false
    let is_xcc_mask = ((index == 0xCC) as usize).wrapping_neg() & 3;

    // 2. Extract bit and shift based on lane
    // Lane 1: special=0, bit becomes 1
    // Lane 3: special=1, bit becomes 2
    assert!(lane < 4);
    let val = match lane {
        0 => reg[0],
        1 => reg[1],
        2 => reg[2],
        3 => reg[3],
        _ => unreachable!(),
    };
    let bit = (val >> bit_within_lane) & 1;
    let res = (bit << (lane >> 1)) as usize;

    // 3. Apply the Lane-based filter
    // final_mask is ALL_ONES for lanes 1 & 3, else 0
    let final_mask = (lane & 1).wrapping_neg();
    let lane_filtered = res & final_mask;

    // 4. Merge the XCC override
    // If is_xcc_mask is ALL_ONES, result becomes 3.
    // Otherwise, result stays as lane_filtered (0, 1, or 2).
    let result = lane_filtered | is_xcc_mask;
    assert!(result == 0 || result == 1 || result == 2 || result == 3);

    unsafe { std::mem::transmute(result as u64) }
}
