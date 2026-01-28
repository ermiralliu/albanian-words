use std::simd::{
    Simd,
    cmp::{SimdPartialEq, SimdPartialOrd},
};

const SIMD_BYTESIZE: usize = 32;

const _SIMD_BITSIZE: usize = SIMD_BYTESIZE * 8;

type SimdHere = Simd<u8, SIMD_BYTESIZE>;

pub struct ChunkProcessor<'a, F>
where
    F: FnMut(&[u8]),
{
    slic: &'a [u8],
    process_word: F,
    word_start: Option<usize>,
}

impl<'a, F> ChunkProcessor<'a, F>
where
    F: FnMut(&[u8]),
{
    #[inline(always)]
    pub fn new(slic: &'a [u8], process_word: F) -> Self {
        Self {
            slic,
            process_word,
            word_start: None,
        }
    }

    // This is your 'reuse_processing_loop', adapted to be a method
    pub fn process_mask(&mut self, mask: u64, chunk_base: usize) {
        let is_in_word = self.word_start.is_some();
        let (indices, info) = self.read_word_from_bitset(mask, is_in_word);

        let count = (info >> 1) as usize;
        let ends_in_word = (info & 1) == 1;

        let mut idx = 0;

        // 1. Finish pending word from previous chunk
        if let Some(start_idx) = self.word_start {
            // If we were in a word, the first index must be an end.
            // If count == 0, the word continues through this whole chunk.
            if idx < count {
                let end_rel = unsafe { *indices.get_unchecked(idx) } as usize;
                let word_end = chunk_base + end_rel;

                // SAFETY: Logic guarantees bounds
                let word = unsafe { self.slic.get_unchecked(start_idx..word_end) };
                (self.process_word)(word);

                self.word_start = None;
                idx += 1;
            } else {
                return;
            }
        }

        // 2. Process complete words entirely within this chunk
        // We now have pairs of (Start, End)
        while idx + 1 < count {
            let start_rel = unsafe { *indices.get_unchecked(idx) } as usize;
            let end_rel = unsafe { *indices.get_unchecked(idx + 1) } as usize;

            let abs_start = chunk_base + start_rel;
            let abs_end = chunk_base + end_rel;

            let word = unsafe { self.slic.get_unchecked(abs_start..abs_end) };
            (self.process_word)(word);

            idx += 2;
        }

        // 3. Start new word (if the last transition was a start)
        if ends_in_word {
            // The last index is a start
            let start_rel = unsafe { *indices.get_unchecked(count - 1) } as usize;
            self.word_start = Some(chunk_base + start_rel);
        }
    }

    // Your pipelined bitset reader
    #[inline(always)]
    pub fn read_word_from_bitset(&self, mut mask: u64, is_in_word: bool) -> ([u8; 16], u8) {
        const MASK_NEG_BITS: u64 = (1 << SIMD_BYTESIZE) - 1;
        let mut neg_mask = (!mask) & MASK_NEG_BITS;
        let mut indices = [0u8; 16];
        let mut current_state = is_in_word;
        let mut index_count = 0;

        loop {
            let (active_mask, other_mask) = if current_state {
                (&mut neg_mask, &mut mask)
            } else {
                (&mut mask, &mut neg_mask)
            };

            let idx = active_mask.trailing_zeros() as usize;
            if idx >= SIMD_BYTESIZE {
                break;
            }

            // SAFETY: index_count max is 16 based on loop bounds/logic
            unsafe { *indices.get_unchecked_mut(index_count) = idx as u8 };
            index_count += 1;

            if index_count >= 16 {
                break;
            } // Safety break, though unlikely with 64-bit chunks

            current_state = !current_state;

            // Clear the bit we just found to find the next one
            // We can clear everything up to idx + 1 to ensure progress
            *other_mask &= !((1 << (idx + 1)) - 1);
        }

        (indices, (current_state as u8) + ((index_count as u8) << 1))
    }

    pub fn process_streaming_alternative(&mut self)
    where
        F: FnMut(&[u8]),
    {
        // Initialize struct
        // let mut processor = ChunkProcessor::new(slic, process_word);

        let num_full_chunks = self.slic.len() / SIMD_BYTESIZE;
        let mut ends_with_c3 = 0;

        // --- Main Loop ---
        for chunk_idx in 0..num_full_chunks {
            let chunk_start = chunk_idx * SIMD_BYTESIZE;

            // Note: process_chunk acts on mutable slice data, but processor holds immutable ref.
            // We must perform the unsafe read/write carefully or split the slice beforehand.
            // Assuming process_chunk reads/writes in place and returns a mask.
            let (mask, ends_with) = unsafe {
                let ptr = self.slic.as_ptr().add(chunk_start) as *mut u8;
                let slice_mut = std::slice::from_raw_parts_mut(ptr, SIMD_BYTESIZE);
                process_chunk(slice_mut, ends_with_c3)
            };
            ends_with_c3 = ends_with;

            self.process_mask(mask, chunk_start);
        }

        // --- Tail Handling ---
        let last_chunk_start = num_full_chunks * SIMD_BYTESIZE;
        let remaining = self.slic.len() - last_chunk_start;

        if remaining > 0 {
            let mut buffer = [0u8; SIMD_BYTESIZE];
            let last_slice = unsafe { self.slic.get_unchecked(last_chunk_start..) };

            buffer[..remaining].copy_from_slice(last_slice);

            let (mask, _) = unsafe { process_chunk(&mut buffer, ends_with_c3) };

            // Zero out garbage bits from the padding so we don't read past end of slice
            let valid_mask_bits = if remaining >= 64 { !0 } else { (1u64 << remaining) - 1 };

            self.process_mask(mask & valid_mask_bits, last_chunk_start);
        }
    }
}

#[target_feature(enable = "avx2")]
fn process_chunk(slic: &mut [u8], prev_ends_with_c3: u64) -> (u64, u64) {
    // 16 byte
    if slic.len() != SIMD_BYTESIZE {
        unsafe {
            std::hint::unreachable_unchecked();
        }
    }
    const SIMD_BYTESIZE_SHIFT: usize = SIMD_BYTESIZE - 1;

    let chunk = SimdHere::from_slice(slic);
    // 1. Mark 0xC3 early
    let is_c3 = chunk.simd_eq(SimdHere::splat(0xC3));
    let c3_mask = is_c3.to_bitmask();
    let is_last_c3 = c3_mask >> SIMD_BYTESIZE_SHIFT; // this is not used. it's returned. We only use the previous one
    //
    // 2. Transform the chunk: OR 0x20 everywhere except 0xC3 lanes
    let lower_hack = chunk | SimdHere::splat(0x20);
    let transformed = is_c3.select(chunk, lower_hack); // Uses vector blend, much faster than casting masks

    // let not_c3_mask = !is_c3;
    // let or_vec = not_c3_mask.to_int().cast::<u8>() & SimdHere::splat(0x20);
    // let transformed = chunk | or_vec;

    // 3. Range check the ALREADY transformed data
    let is_digit = transformed.simd_ge(SimdHere::splat(b'0')) & transformed.simd_le(SimdHere::splat(b'9'));
    let is_alpha = transformed.simd_ge(SimdHere::splat(b'a')) & transformed.simd_le(SimdHere::splat(b'z'));

    let is_alnum = is_digit | is_alpha;

    // 4. Build the valid_mask
    let valid_mask = is_alnum.to_bitmask() | c3_mask | (c3_mask << 1) | prev_ends_with_c3 as u64;

    // 5. Write back the transformed data
    transformed.copy_to_slice(&mut slic[0..SIMD_BYTESIZE]);

    (valid_mask, is_last_c3)
}
