// use std::simd::prelude::*;
//
// pub const ARR: [u64; 4] = [
//     // the printed bits need to be reversed for some reason
//     0b0000000000000000000000000000000000000000000000001111111111000000u64.reverse_bits(),
//     0b0111111111111111111111111110000001111111111111111111111111100000u64.reverse_bits(),
//     0b0000000000000000000000000000000000000000000000000000000000000000u64.reverse_bits(),
//     0b0001000000000000000000000000000000000000000000000000000000000000u64.reverse_bits(),
// ];
//
// #[repr(u64)]
// #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
// pub enum FirstPassType {
//     Skip = 0,
//     Number = 1,
//     Letter = 2,
// }
//
// #[repr(u64)]
// #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
// pub enum SecondPassType {
//     END = 0,
//     Letter = 1,
//     XC3 = 2,
//     XCC = 3,
// }
//
// #[inline(always)]
// pub fn get_first_pass_type(index: u8, reg: u64x4) -> FirstPassType {
//     let index = index as usize;
//     // let lane = index / 64;
//     // let bit_within_lane = index % 64;
//     // Just to make sure at this point *crying emoji*
//     let lane = index >> 6;
//     let bit_within_lane = index & 63;
//
//     assert!(lane < 4);
//     let val = match lane {
//         0 => reg[0],
//         1 => reg[1],
//         2 => reg[2],
//         3 => reg[3],
//         _ => unreachable!(),
//     };
//
//     let bucket_type_shift = lane % 2;
//     let a = (val >> bit_within_lane) & 1;
//     let a = a << bucket_type_shift;
//     assert!(a == 0 || a == 1 || a == 2);
//     unsafe { std::mem::transmute(a) }
// }
//
// #[inline(always)]
// pub fn get_second_pass_type(index: u8, reg: u64x4) -> SecondPassType {
//     let index = index as usize;
//     // let lane = index / 64;
//     // let bit_within_lane = index % 64;
//     let lane = index >> 6;
//     let bit_within_lane = index & 63;
//
//     // 1. Check for 0xCC: all ones if true, all zeros if false
//     let is_xcc_mask = ((index == 0xCC) as usize).wrapping_neg() & 3;
//
//     // 2. Extract bit and shift based on lane
//     // Lane 1: special=0, bit becomes 1
//     // Lane 3: special=1, bit becomes 2
//     assert!(lane < 4);
//     let val = match lane {
//         0 => reg[0],
//         1 => reg[1],
//         2 => reg[2],
//         3 => reg[3],
//         _ => unreachable!(),
//     };
//     let bit = (val >> bit_within_lane) & 1;
//     let res = (bit << (lane >> 1)) as usize;
//
//     // 3. Apply the Lane-based filter
//     // final_mask is ALL_ONES for lanes 1 & 3, else 0
//     let final_mask = (lane & 1).wrapping_neg();
//     let lane_filtered = res & final_mask;
//
//     // 4. Merge the XCC override
//     // If is_xcc_mask is ALL_ONES, result becomes 3.
//     // Otherwise, result stays as lane_filtered (0, 1, or 2).
//     let result = lane_filtered | is_xcc_mask;
//     assert!(result == 0 || result == 1 || result == 2 || result == 3);
//
//     unsafe { std::mem::transmute(result as u64) }
// }
//

use std::{
    simd::cmp::{SimdPartialEq, SimdPartialOrd},
};

use crate::{SIMD_BYTESIZE, SimdHere};

// #[target_feature(enable = "avx2")]
// fn process_chunk(slic: &mut [u8], prev_ends_with_c3: u64) -> (u64, u64) {
//     // 16 byte
//     if slic.len() != SIMD_BYTESIZE {
//         unsafe {
//             std::hint::unreachable_unchecked();
//         }
//     }
//     const SIMD_BYTESIZE_SHIFT: usize = SIMD_BYTESIZE - 1;

//     let chunk = SimdHere::from_slice(slic);
//     // 1. Mark 0xC3 early
//     let is_c3 = chunk.simd_eq(SimdHere::splat(0xC3));
//     let c3_mask = is_c3.to_bitmask();
//     let is_last_c3 = c3_mask >> SIMD_BYTESIZE_SHIFT; // this is not used. it's returned. We only use the previous one
//     //
//     // 2. Transform the chunk: OR 0x20 everywhere except 0xC3 lanes
//     let lower_hack = chunk | SimdHere::splat(0x20);
//     let transformed = is_c3.select(chunk, lower_hack); // Uses vector blend, much faster than casting masks

//     // 3. Range check the ALREADY transformed data
//     let is_digit = transformed.simd_ge(SimdHere::splat(b'0')) & transformed.simd_le(SimdHere::splat(b'9'));
//     let is_alpha = transformed.simd_ge(SimdHere::splat(b'a')) & transformed.simd_le(SimdHere::splat(b'z'));

//     let is_alnum = is_digit | is_alpha;

//     // 4. Build the valid_mask
//     let valid_mask = is_alnum.to_bitmask() | c3_mask | (c3_mask << 1) | prev_ends_with_c3 as u64;

//     // 5. Write back the transformed data
//     transformed.copy_to_slice(&mut slic[0..SIMD_BYTESIZE]);

//     (valid_mask, is_last_c3)
// }

const LANES_PER_U64: usize = 64 / SIMD_BYTESIZE;


#[target_feature(enable = "avx2")]
fn process_chunk(full_slic: &mut [u8], mut prev_ends_with_c3: u64) -> (u64, u64) {
    // Ensure we have exactly 64 bytes
    debug_assert_eq!(full_slic.len(), 64);
    
    let mut combined_mask: u64 = 0;
    const SIMD_BYTESIZE_SHIFT: usize = SIMD_BYTESIZE - 1;

    for i in 0..LANES_PER_U64 {
        let start = i * SIMD_BYTESIZE;
        let slic = &mut full_slic[start..start + SIMD_BYTESIZE];
        
        // Manual hint for the compiler to elide bounds checks
        if slic.len() != SIMD_BYTESIZE { unsafe { std::hint::unreachable_unchecked(); } }

        let chunk = SimdHere::from_slice(slic);
        
        // 1. Identify 0xC3
        let is_c3 = chunk.simd_eq(SimdHere::splat(0xC3));
        let c3_mask = is_c3.to_bitmask() as u64;
        let is_last_c3 = (c3_mask >> SIMD_BYTESIZE_SHIFT) & 1 != 0;

        // 2. Transform (Lowercase hack)
        let lower_hack = chunk | SimdHere::splat(0x20);
        let transformed = is_c3.select(chunk, lower_hack);

        // 3. Validation
        let is_digit = transformed.simd_ge(SimdHere::splat(b'0')) & transformed.simd_le(SimdHere::splat(b'9'));
        let is_alpha = transformed.simd_ge(SimdHere::splat(b'a')) & transformed.simd_le(SimdHere::splat(b'z'));
        let is_alnum = is_digit | is_alpha;

        // 4. Build local mask and shift into the combined u64
        // Logic: (Alnum | C3 | Follower-of-C3 | Carry-over)
        let local_valid = is_alnum.to_bitmask() as u64 
                          | c3_mask 
                          | (c3_mask << 1) 
                          | (prev_ends_with_c3 as u64);

        // Place the local bits into the correct position in the 64-bit mask
        combined_mask |= local_valid << (i * SIMD_BYTESIZE);

        // 5. Write back
        transformed.copy_to_slice(slic);

        // Carry the last lane's C3 status to the next SIMD chunk
        prev_ends_with_c3 = is_last_c3 as u64;
    }

    (combined_mask, prev_ends_with_c3)
}

pub fn process_streaming_new<F>(slic: &mut [u8], process_word: &mut F)
where
    F: FnMut(&[u8]),
{
    let mut word_start = Option::None;

    let num_full_chunks = slic.len() / SIMD_BYTESIZE;
    let mut ends_with_c3 = 0; // this is basically a boolean, but we don't want to pay for that
    let mut last_was_valid = false;

    for chunk_idx in (0..num_full_chunks).step_by(LANES_PER_U64) {
        let chunk_start = chunk_idx * SIMD_BYTESIZE;
        let chunk = &mut slic[chunk_start..chunk_start + 64];

        let (mask, ends_with) = unsafe { process_chunk(chunk, ends_with_c3) };
        ends_with_c3 = ends_with;
        read_word_from_bitset_new(
            mask,
            chunk_start as u32,
            last_was_valid,
            slic,
            &mut word_start,
            process_word,
        );
        last_was_valid = (mask >> (SIMD_BYTESIZE - 1)) & 1 == 1;
    }

    // let mut buffer = [0u8; SIMD_BYTESIZE]; // Pre-filled with 0s

    // let last_chunk_start = num_full_chunks * SIMD_BYTESIZE;
    // let last_chunk = &slic[last_chunk_start..];
    // // Copy small_data into the beginning of the buffer
    // if last_chunk.len() >= SIMD_BYTESIZE {
    //     unsafe {
    //         unreachable_unchecked();
    //     }
    // }
    // buffer[..last_chunk.len()].copy_from_slice(last_chunk);
    // let mask = unsafe { process_chunk(&mut buffer, ends_with_c3) };
    // read_word_from_bitset_new(
    //     mask.0,
    //     last_chunk_start as u32,
    //     last_was_valid,
    //     slic,
    //     &mut word_start,
    //     process_word,
    // );
}

// #[inline(always)]
fn read_word_from_bitset_new<F>(
    mask: u64,
    chunk_start: u32,
    last_was_valid: bool,
    slic: &[u8], // we can pass only the start pointer here.
    word_start: &mut Option<u32>,
    process_word: &mut F,
) where
    F: FnMut(&[u8]),
{
    let a = (mask << 1) | last_was_valid as u64; // Here it should be OR-ed with the previous byte of the previous bitset
    let b = a | mask;
    let mut ends = mask ^ b; // each 0 after a 1 is written as 1 here, so we have the word ends
    if let Some(previous_start) = word_start {
        let previous_start = *previous_start as usize;
        let end = ends.trailing_zeros();
        let end = if end as usize >= SIMD_BYTESIZE {
            SIMD_BYTESIZE
        } else {
            end as usize
        }; // This condition should be good for safety
        let end = end + chunk_start as usize;
        let word = unsafe { slic.get_unchecked(previous_start..end) };
        process_word(word);
        ends &= ends - 1; // this flips the least significant bit
    }
    let mut starts = b ^ a; // now the starts are written with 1 here

    loop {
        let start = starts.trailing_zeros() as usize;
        let end = ends.trailing_zeros() as usize;
        if start >= SIMD_BYTESIZE {
            *word_start = None; // I has forgotten this initially
            return;
        }
        if end >= SIMD_BYTESIZE {
            *word_start = Some(chunk_start + start as u32);
            return;
        }
        starts &= starts - 1;
        ends &= ends - 1;
        let start = start + chunk_start as usize;
        let end = end + chunk_start as usize;

        let word = unsafe { slic.get_unchecked(start..end) };
        process_word(word);
    }
}
