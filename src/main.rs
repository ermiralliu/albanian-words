#![feature(portable_simd)]
use std::{
    hint::unreachable_unchecked,
    simd::{
        Simd,
        cmp::{SimdPartialEq, SimdPartialOrd},
    },
};

const SIMD_BYTESIZE: usize = 32;

const _SIMD_BITSIZE: usize = SIMD_BYTESIZE * 8;

type SimdHere = Simd<u8, SIMD_BYTESIZE>;
// type ProcessWordFn = fn(&[u8]) -> ();

pub mod alb_parser;
pub mod bitset;
pub mod chunk_processor;
pub mod file_readers;
pub mod properties;
pub mod stop_words;

use std::{collections::HashMap, sync::Mutex, thread, time::Instant, vec};

use alb_parser::AlbanianParser;
use file_readers::seq_read;
use properties::Properties;
use rustc_hash::FxBuildHasher;
use std::env;

use crate::chunk_processor::ChunkProcessor;

const DEFAULT_VEC_CAPACITY: usize = 256 * 1024;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Hello, world!");
    // let verbs = vec!["punojmë", "punuam", "shkruar", "lexuar", "vendosur"];
    let vocab_vector: Vec<&[u8]> = vec![b"punoj", b"shkruaj", b"lexoj", b"vendos"]; // this is just for tests,
    let mut map: HashMap<&[u8], u16, FxBuildHasher> = HashMap::default();
    for (i, &word) in vocab_vector.iter().enumerate() {
        map.insert(word, i as u16);
    }
    let config = {
        let config_file = env::var("CONFIG_FILE").unwrap_or("./config.ini".to_string());
        match Properties::try_from_config_file(&config_file) {
            Ok(props) => props,
            Err(error) => {
                eprintln!("Configuration file not found or invalid values found: {:?}", error);
                return Ok(()); // Yeah I should return something worthwhile here
            }
        }
    };
    println!("Config information: {:#?}", config);

    let start = Instant::now();

    let sr = seq_read::SequentialFileReader::try_new(&config.article_file, config.article_separator)?;
    // let mut count = 0;
    // let set: HashSet<&[u8]> = STOP_WORDS.iter().copied().map(|word| word.as_bytes()).collect();

    // let reg = u64x4::from_array(ARR);

    // These live on the stack of main
    let shared_reader = Mutex::new(sr);
    // 'set' and 'parser' can just be regular references
    let core_count = num_cpus::get_physical();
    // let core_count = 1;

    let final_tokens: Vec<Vec<u16>> = thread::scope(|s| {
        let mut handles = vec![];

        for _ in 0..core_count {
            // We borrow from the outer scope
            let r = &shared_reader;
            // let stop_words = &set;
            let mut local_parser: AlbanianParser<'_, FxBuildHasher> = AlbanianParser::new(&map);

            let h = s.spawn(move || {
                let mut local_buf = Vec::with_capacity(DEFAULT_VEC_CAPACITY);
                let mut thread_results = Vec::with_capacity(128);

                loop {
                    local_buf.clear();
                    // Lock the reader just to fill the buffer
                    {
                        let mut reader = r.lock().unwrap();
                        if !reader.read_into(&mut local_buf) {
                            break;
                        }
                    }

                    let mut article_tokens = Vec::with_capacity(256);
                    let closure = &mut |word: &[u8]| {
                        if
                        /* stop_words.contains(word) || */
                        word[0].is_ascii_digit() {
                            return;
                        }
                        #[cfg(debug_assertions)]
                        {
                            let printable_word = unsafe { std::str::from_utf8_unchecked(word) };
                            println!("Word: {}", printable_word);
                        }
                        let id = local_parser.single_verb_to_base(word);
                        article_tokens.push(id.unwrap_or(0));
                    };
                    let mut chunk_processor = ChunkProcessor::new(&mut local_buf, closure);
                    chunk_processor.process_streaming_alternative();

                    // process_streaming_alternative(&mut local_buf,
                    if !article_tokens.is_empty() {
                        thread_results.push(article_tokens);
                    }
                }
                thread_results
            });
            handles.push(h);
        }

        // Join is automatic at the end of the scope, but we collect results here
        handles.into_iter().flat_map(|h| h.join().unwrap()).collect()
    });

    let end = Instant::now();
    println!("{:?}", final_tokens[final_tokens.len() - 1]);
    println!("Time passed: {:?}", (end - start));
    Ok(())
}

// #[inline(always)]
// #[target_feature(enable = "avx2")]
// fn process_chunk(slic: &mut [u8], prev_ends_with_c3: u64) -> (u64, u64) {
//     // 16 byte
//     if slic.len() != SIMD_BYTESIZE {
//         unsafe {
//             unreachable_unchecked();
//         }
//     }
//     const SIMD_BYTESIZE_SHIFT: usize = SIMD_BYTESIZE - 1;
//
//     let chunk = SimdHere::from_slice(slic);
//     // 1. Mark 0xC3 early
//     let is_c3 = chunk.simd_eq(SimdHere::splat(0xC3));
//     let c3_mask = is_c3.to_bitmask();
//     let is_last_c3 = c3_mask >> SIMD_BYTESIZE_SHIFT; // this is not used. it's returned. We only use the previous one
//     //
//     // 2. Transform the chunk: OR 0x20 everywhere except 0xC3 lanes
//     let lower_hack = chunk | SimdHere::splat(0x20);
//     let transformed = is_c3.select(chunk, lower_hack); // Uses vector blend, much faster than casting masks
//
//     // let not_c3_mask = !is_c3;
//     // let or_vec = not_c3_mask.to_int().cast::<u8>() & SimdHere::splat(0x20);
//     // let transformed = chunk | or_vec;
//
//     // 3. Range check the ALREADY transformed data
//     let is_digit = transformed.simd_ge(SimdHere::splat(b'0')) & transformed.simd_le(SimdHere::splat(b'9'));
//     let is_alpha = transformed.simd_ge(SimdHere::splat(b'a')) & transformed.simd_le(SimdHere::splat(b'z'));
//
//     let is_alnum = is_digit | is_alpha;
//
//     // 4. Build the valid_mask
//     let valid_mask = is_alnum.to_bitmask() | c3_mask | (c3_mask << 1) | prev_ends_with_c3 as u64;
//
//     // 5. Write back the transformed data
//     transformed.copy_to_slice(&mut slic[0..SIMD_BYTESIZE]);
//
//     (valid_mask, is_last_c3)
// }

// #[inline(always)]
// fn process_streaming<F>(slic: &mut [u8], process_word: &mut F)
// where
//     F: FnMut(&[u8]),
// {
//     let mut word_start = Option::None;
//
//     let num_full_chunks = slic.len() / SIMD_BYTESIZE;
//     let mut ends_with_c3 = 0; // this is basically a boolean, but we don't want to pay for that
//
//     for chunk_idx in 0..num_full_chunks {
//         let chunk_start = chunk_idx * SIMD_BYTESIZE;
//         let chunk = &mut slic[chunk_start..chunk_start + SIMD_BYTESIZE];
//
//         let (mask, ends_with) = unsafe { process_chunk(chunk, ends_with_c3) };
//         ends_with_c3 = ends_with;
//         read_word_from_bitset(mask, chunk_start, slic, &mut word_start, process_word);
//     }
//
//     let mut buffer = [0u8; SIMD_BYTESIZE]; // Pre-filled with 0s
//
//     let last_chunk_start = num_full_chunks * SIMD_BYTESIZE;
//     let last_chunk = &slic[last_chunk_start..];
//     // Copy small_data into the beginning of the buffer
//     assert!(last_chunk.len() < SIMD_BYTESIZE);
//     if last_chunk.len() >= SIMD_BYTESIZE {
//         unsafe {
//             unreachable_unchecked();
//         }
//     }
//     buffer[..last_chunk.len()].copy_from_slice(last_chunk);
//     let mask = unsafe { process_chunk(&mut buffer, ends_with_c3) };
//     read_word_from_bitset(mask.0, last_chunk_start, slic, &mut word_start, process_word);
// }
//
// // #[inline(always)]
// fn read_word_from_bitset<F>(
//     mut mask: u64,
//     chunk_start: usize,
//     slic: &[u8],
//     word_start: &mut Option<usize>,
//     process_word: &mut F,
// ) where
//     F: FnMut(&[u8]),
// {
//     const MASK_NEG_BITS: u64 = (1 << SIMD_BYTESIZE) - 1;
//     let mut neg_mask = (!mask) & MASK_NEG_BITS;
//
//     loop {
//         // 1. Determine the mask
//         let (active_mask, other_mask) = match word_start {
//             Some(_) => (&mut neg_mask, &mut mask),
//             None => (&mut mask, &mut neg_mask),
//         };
//
//         // 2. Find the transition
//         let idx = active_mask.trailing_zeros() as usize;
//         if idx >= SIMD_BYTESIZE {
//             return;
//         }
//
//         // 3. Action based on state
//         if let Some(st) = *word_start {
//             let word = unsafe { slic.get_unchecked(st..chunk_start + idx) };
//             process_word(word);
//             *word_start = None;
//         } else {
//             *word_start = Some(chunk_start + idx);
//         }
//
//         *other_mask &= zero_everything_before(idx);
//     }
// }

// fn process_streaming_alternative<F>(slic: &mut [u8], process_word: &mut F)
// where
//     F: FnMut(&[u8]),
// {
//     let mut word_start = Option::None;
//
//     let num_full_chunks = slic.len() / SIMD_BYTESIZE;
//     let mut ends_with_c3 = 0; // this is basically a boolean, but we don't want to pay for that
//
//     for chunk_idx in 0..num_full_chunks {
//         let chunk_start = chunk_idx * SIMD_BYTESIZE;
//
//         let (mask, ends_with) = unsafe {
//             process_chunk(
//                 &mut slic.get_unchecked_mut(chunk_start..chunk_start + SIMD_BYTESIZE),
//                 ends_with_c3,
//             )
//         };
//         ends_with_c3 = ends_with;
//         reuse_processing_loop(slic, chunk_idx, &mut ends_with_c3, &mut word_start, process_word);
//     }
//
//     let mut buffer = [0u8; SIMD_BYTESIZE]; // Pre-filled with 0s
//
//     let last_chunk_start = num_full_chunks * SIMD_BYTESIZE;
//     let last_chunk = &slic[last_chunk_start..];
//     // Copy small_data into the beginning of the buffer
//     assert!(last_chunk.len() < SIMD_BYTESIZE);
//     if last_chunk.len() >= SIMD_BYTESIZE {
//         unsafe {
//             std::hint::unreachable_unchecked();
//         }
//     }
//     buffer[..last_chunk.len()].copy_from_slice(last_chunk);
//     let mask = unsafe { process_chunk(&mut buffer, ends_with_c3) };
//     reuse_processing_loop(slic, last_chunk_start, &mut ends_with_c3, &mut word_start, process_word);
// }
//
// #[inline]
// fn reuse_processing_loop<F>(
//     slic: &mut [u8],
//     chunk_idx: usize,
//     ends_with_c3: &mut u64,
//     word_start: &mut Option<usize>,
//     process_word: &mut F,
// ) where
//     F: FnMut(&[u8]),
// {
//     let (arr, count_and_is_next) = read_word_from_bitset_alternative(mask, word_start.is_none());
//     let length = (count_and_is_next >> 1) as usize;
//     let mut start = 0;
//     let is_next_in_word = (count_and_is_next & 1) == 1;
//     if let Some(start_idx) = *word_start {
//         let word_end = unsafe { *arr.get_unchecked(0) } as usize;
//         let word = unsafe { &slic.get_unchecked(start_idx..word_end) };
//         process_word(word);
//         start = 1;
//     }
//     let chunk = unsafe { slic.get_unchecked(chunk_start..chunk_start + SIMD_BYTESIZE) };
//     for i in (start..length).step_by(2) {
//         let word_start = unsafe { *arr.get_unchecked(i) as usize };
//         let word_end = unsafe { *arr.get_unchecked(i + 1) } as usize;
//         let word = unsafe { &chunk.get_unchecked(word_start..word_end + 1) };
//         process_word(word);
//     }
//     *word_start = if is_next_in_word {
//         let place = unsafe { *arr.get_unchecked(length - 1) } as usize;
//         Some(chunk_start + place)
//     } else {
//         None
//     }
// }
//
// fn read_word_from_bitset_alternative(mut mask: u64, is_in_word: bool) -> ([u8; 16], u8) {
//     // I think this is so simple now, that it should get instantly hyperoptimized. Like, this is
//     // the best kinda shit for pipelining
//     const MASK_NEG_BITS: u64 = (1 << SIMD_BYTESIZE) - 1;
//     let mut neg_mask = (!mask) & MASK_NEG_BITS;
//     let mut indices = [0u8; 16];
//     let mut is_in_word = is_in_word;
//     let mut index_count = 0;
//
//     loop {
//         // 1. Determine the mask
//         let (active_mask, other_mask) = if is_in_word {
//             (&mut neg_mask, &mut mask)
//         } else {
//             (&mut mask, &mut neg_mask)
//         };
//
//         // 2. Find the transition
//         let idx = active_mask.trailing_zeros() as usize;
//         if idx >= SIMD_BYTESIZE {
//             break;
//         }
//         if index_count > 15 {
//             unsafe { std::hint::unreachable_unchecked() };
//         }
//         // Add new delta
//         indices[index_count] = idx as u8;
//         index_count += 1;
//
//         // 3. Action based on state
//         is_in_word = !is_in_word;
//
//         *other_mask &= zero_everything_before(idx);
//     }
//     (indices, is_in_word as u8 + ((index_count as u8) << 1))
// }
//
// #[inline(always)]
// fn zero_everything_before(idx: usize) -> u64 {
//     return !((1 << (idx + 1)) - 1);
// }
