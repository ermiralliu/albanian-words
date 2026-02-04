use std::{
    simd::cmp::{SimdPartialEq, SimdPartialOrd},
};

use crate::{SIMD_BYTESIZE, SimdHere};

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
        last_was_valid = (mask >> (64 - 1)) & 1 == 1; // 63 is placing 63 in 0
    }
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
        let end = if end as usize == 64 { // I should not hardcode 64 but we'll see
            64
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
        if start == 64 {
            *word_start = None; // I has forgotten this initially
            return;
        }
        if end >= 64 {
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
