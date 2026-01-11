pub mod alb_parser;
pub mod file_readers;
pub mod stop_words;
use std::{
    collections::{HashMap, HashSet}, fmt::write, sync::Mutex, thread, time::Instant, vec
};

use alb_parser::AlbanianParser;
use file_readers::seq_read;
use stop_words::STOP_WORDS;

const WORD_DELIMITER_BITSET: [bool; 256] = {
    // These are the single character nes
    let mut init = [false; 256];
    let mut i = 0;

    while i < 256 {
        match i {
            0x00..=0x20 => init[i] = true,
            // ASCII Punctuation (Excluding 0-9 which is 0x30-0x39)
            0x21..=0x2F | 0x3A..=0x40 | 0x5B..=0x60 | 0x7B..=0x7F => init[i] = true,
            _ => {}
        }
        i += 1;
    }
    init
};

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum CharClass {
    // I had forgotten I could do this and that it's the best possible way of handling such cases.
    // Delimiter = 0, // Space, punctuation, etc. // Why would i handle these differently?
    // Ordering these by probability but that probably doesn't matter here ngl
    Lower = 0,    // a-z
    Upper = 1,    // A-Z
    Other = 2,    // Mostly whitespaces and punctuation and other stuff
    Number = 3,   // 0-9
    C3Prefix = 4, // 0xC3 (ë, ç)
    CCPrefix = 5, // 0xCC (Combining marks)
    Byte3 = 6,    // We can instantly skip 3 bytes for these
    Byte4 = 7,    // We can skip 4 for these
    StillNumber = 8, // If is_number, these are just delimiters
                  // E2Prefix = 6,  // 0xE2 (Smart quotes)
                  // Foreign = 7,   // Everything else (ö, ü, Chinese, etc.)
} // I'm giving up on the hyphen

const GET_CHAR_TYPE: [CharClass; 256] = {
    let mut init = [CharClass::Other; 256];
    let mut i: u8 = 0;

    loop {
        match i {
            b'0'..=b'9' => init[i as usize] = CharClass::Number,
            b'A'..=b'Z' => init[i as usize] = CharClass::Upper,
            b'a'..=b'z' => init[i as usize] = CharClass::Lower,
            0xE0..=0xEF => init[i as usize] = CharClass::Byte3,

            // THE 4-BYTE ZONE (11110xxx)
            0xF0..=0xF7 => init[i as usize] = CharClass::Byte4,
            _ => {}
        }
        if i == 255 {
            break;
        }
        i += 1;
    }
    init[b'.' as usize] = CharClass::StillNumber;
    init[b',' as usize] = CharClass::StillNumber;
    init[b'\'' as usize] = CharClass::StillNumber;
    // init[b'-' as usize] = CharClass::Minus;
    init[0xC3] = CharClass::C3Prefix; // To handle extended latin, to invalidate or get e and c 
    init[0xCC] = CharClass::CCPrefix; // Individual diaeresis 0x88, cedilla 0xA7
    // init[0xE2] = true;
    init
};

const DEFAULT_VEC_CAPACITY: usize = 256 * 1024;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Hello, world!");
    // let verbs = vec!["punojmë", "punuam", "shkruar", "lexuar", "vendosur"];
    let vocab_vector: Vec<&[u8]> = vec![b"punoj", b"shkruaj", b"lexoj", b"vendos"]; // this is just for tests,
    let mut map = HashMap::new();
    for (i, &word) in vocab_vector.iter().enumerate() {
        map.insert(word, i as u16);
    }

    let start = Instant::now();

    let mut sr = seq_read::SequentialFileReader::try_new(
        "/home/ermir/Documents/Diploma/csv_processing/finalized-content/test_content.txt",
        b'\x1E',
    )?;
    // let mut count = 0;
    let set: HashSet<&[u8]> = STOP_WORDS.iter().copied().map(|word| word.as_bytes()).collect();

    // These live on the stack of main
    let shared_reader = Mutex::new(sr);
    // 'set' and 'parser' can just be regular references
    let core_count = num_cpus::get_physical();

    let final_tokens: Vec<Vec<u16>> = thread::scope(|s| {
        let mut handles = vec![];

        for _ in 0..core_count {
            // We borrow from the outer scope
            let r = &shared_reader;
            let stop_words = &set;
            let mut local_parser = AlbanianParser::new(&map);

            let h = s.spawn(move || {
                let mut local_buf = Vec::with_capacity(DEFAULT_VEC_CAPACITY);
                let mut thread_results = Vec::new();

                loop {
                    local_buf.clear();
                    // Lock the reader just to fill the buffer
                    {
                        let mut reader = r.lock().unwrap();
                        if !reader.read_into(&mut local_buf) {
                            break;
                        }
                    }

                    let mut article_tokens = Vec::new();
                    let mut cursor = 0;

                    while let Some((start, end, is_num)) = next_word_inplace(&mut local_buf, &mut cursor) {
                        if end - start < 2 {
                            continue;
                        }
                        if is_num {
                            continue;
                        }

                        let word = &local_buf[start..end];
                        if stop_words.contains(word) {
                            continue;
                        }

                        if let Some(nr) = local_parser.single_verb_to_base(word) {
                            article_tokens.push(nr);
                        } else {
                            article_tokens.push(0);
                        }
                    }

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

// fn next_word_inplace(src: &mut [u8], cursor: &mut usize) -> Option<(usize, usize, bool)> {
//     let len = src.len();
//
//     // 1. Skip leading delimiters (including the 3-byte ones)
//     while *cursor < len {
//         let b = src[*cursor];
//
//         // Handle ASCII delimiters
//         if WORD_DELIMITER_BITSET[b as usize] {
//             *cursor += 1;
//             continue;
//         }
//
//         // Handle 3-byte delimiters at the start
//         if b == 0xE2 && *cursor + 2 < len && src[*cursor + 1] == 0x80 {
//             let third = src[*cursor + 2];
//             if third == 0x9C || third == 0x9D || third == 0x93 || third == 0x98 || third == 0x99 {
//                 *cursor += 3;
//                 continue;
//             }
//         }
//
//         break; // Found a non-delimiter byte
//     }
//
//     if *cursor >= len {
//         return None;
//     }
//
//     let word_start = *cursor;
//     let mut write_idx = *cursor;
//     let mut is_numeric = true;
//
//     // 2. Scan and transform
//     while *cursor < len {
//         let b = src[*cursor];
//
//         // ASCII Delimiter check
//         if WORD_DELIMITERS.contains(&b) {
//             // We do NOT increment cursor here; the next call's "skip" logic handles it
//             break;
//         }
//
//         match b {
//             // Rule 1: NFC Normalization (3 -> 2 bytes)
//             0x65 | 0x45 if *cursor + 2 < len && src[*cursor + 1] == 0xCC && src[*cursor + 2] == 0x88 => {
//                 src[write_idx] = 0xC3;
//                 src[write_idx + 1] = 0xAB;
//                 write_idx += 2;
//                 *cursor += 3;
//                 is_numeric = false;
//             }
//
//             // Rule 2: ASCII Lowercase
//             b'A'..=b'Z' => {
//                 src[write_idx] = b + 32;
//                 write_idx += 1;
//                 *cursor += 1;
//                 is_numeric = false;
//             }
//
//             // Rule 3: Albanian Ë/Ç
//             0xC3 if *cursor + 1 < len => {
//                 let next = src[*cursor + 1];
//                 src[write_idx] = 0xC3;
//                 match next {
//                     0x8B | 0xAB => src[write_idx + 1] = 0xAB,
//                     0x87 | 0xA7 => src[write_idx + 1] = 0xA7,
//                     _ => src[write_idx + 1] = next,
//                 }
//                 write_idx += 2;
//                 *cursor += 2;
//                 is_numeric = false;
//             }
//
//             // Rule 4: 3-byte Delimiters (The "Stuck" fix)
//             0xE2 if *cursor + 2 < len && src[*cursor + 1] == 0x80 => {
//                 let third = src[*cursor + 2];
//                 if third == 0x9C || third == 0x9D || third == 0x93 || third == 0x98 || third == 0x99 {
//                     // Stop word here. We don't increment cursor; the skip logic above will jump 3.
//                     break;
//                 } else {
//                     src[write_idx] = b;
//                     write_idx += 1;
//                     *cursor += 1;
//                     is_numeric = false;
//                 }
//             }
//
//             // Rule 5: Standard scan
//             _ => {
//                 if is_numeric && !b.is_ascii_digit() && b != b'.' && b != b'-' {
//                     is_numeric = false;
//                 }
//                 if write_idx != *cursor {
//                     src[write_idx] = b;
//                 }
//                 write_idx += 1;
//                 *cursor += 1;
//             }
//         }
//     }
//
//     if is_numeric && (write_idx == word_start || is_edge_case_not_number(&src[word_start..write_idx])) {
//         is_numeric = false;
//     }
//
//     Some((word_start, write_idx, is_numeric))
// }

// #[inline(always)]
// fn is_edge_case_not_number(s: &[u8]) -> bool {
//     s == b"." || s == b"-" || s == b".."
// }

// fn get_next_word(src: &mut [u8], cursor: &mut usize) -> Option<(usize, usize, bool)> {
//     let len = src.len();
//     let mut is_foreign = false;
//     let mut is_number = false;
//
//     let word_start = {
//         let start = *cursor;
//         while *cursor < len {
//             let b = &src[*cursor];
//
//             match GET_CHAR_TYPE[*b as usize] {
//                 CharClass::Lower => {
//                     start = *cursor;
//                     *cursor+=1;
//                     break;
//                 },
//                 CharClass::Upper => {
//                     *b = *b | 0x20;
//                     start = *cursor;
//                     *cursor+=1;
//                     break;
//                 },
//                 CharClass::Other | CharClass::StillNumber => *cursor += 1,
//                 CharClass::C3Prefix => {
//                     let next_char = &src[*cursor + 1];
//                     match next_char {
//                         0x8B | 0x87 | 0xAB | 0xA7 => {
//                             // lowercase these
//                             *next_char = *next_char | 0x20;
//                         },
//                         _ => {
//                             is_foreign = true;
//                         }
//                     }
//                     start = *cursor;
//                     *cursor += 2;
//                     break;
//                 },
//                 CharClass::Number => {
//                    is_number = true;
//                    start = *cursor;
//                    *cursor += 1;
//                    break;
//                 },
//                 CharClass::CCPrefix => *cursor += 2, // skips pointless diaeresis and cedilla
//                 CharClass::Byte3 => *cursor += 3,
//                 CharClass::Byte4 => *cursor += 4,
//                 _ => unreachable!("Every edge case is caught by Other"),
//             }
//         }
//         start
//     };
//
//     if *cursor >= len {
//         return None;
//     }
//     // This lower part is what I want to fix.
//
//     let word_end = {
//         let mut write_idx = *cursor;
//
//         // 2. Scan and transform
//         while *cursor < len {
//             let b = src[*cursor];
//
//             // ASCII Delimiter check
//             if WORD_DELIMITERS.contains(&b) {
//                 // We do NOT increment cursor here; the next call's "skip" logic handles it
//                 break;
//             }
//
//             match b {
//                 // Rule 1: NFC Normalization (3 -> 2 bytes)
//                 0x65 | 0x45 if *cursor + 2 < len && src[*cursor + 1] == 0xCC && src[*cursor + 2] == 0x88 => {
//                     src[write_idx] = 0xC3;
//                     src[write_idx + 1] = 0xAB;
//                     write_idx += 2;
//                     *cursor += 3;
//                     is_numeric = false;
//                 }
//
//                 // Rule 2: ASCII Lowercase
//                 b'A'..=b'Z' => {
//                     src[write_idx] = b + 32;
//                     write_idx += 1;
//                     *cursor += 1;
//                     is_numeric = false;
//                 }
//
//                 // Rule 3: Albanian Ë/Ç
//                 0xC3 if *cursor + 1 < len => {
//                     let next = src[*cursor + 1];
//                     src[write_idx] = 0xC3;
//                     match next {
//                         0x8B | 0xAB => src[write_idx + 1] = 0xAB,
//                         0x87 | 0xA7 => src[write_idx + 1] = 0xA7,
//                         _ => src[write_idx + 1] = next,
//                     }
//                     write_idx += 2;
//                     *cursor += 2;
//                     is_numeric = false;
//                 }
//
//                 // Rule 4: 3-byte Delimiters (The "Stuck" fix)
//                 0xE2 if *cursor + 2 < len && src[*cursor + 1] == 0x80 => {
//                     let third = src[*cursor + 2];
//                     if third == 0x9C || third == 0x9D || third == 0x93 || third == 0x98 || third == 0x99 {
//                         // Stop word here. We don't increment cursor; the skip logic above will jump 3.
//                         break;
//                     } else {
//                         src[write_idx] = b;
//                         write_idx += 1;
//                         *cursor += 1;
//                         is_numeric = false;
//                     }
//                 }
//
//                 // Rule 5: Standard scan
//                 _ => {
//                     if is_numeric && !b.is_ascii_digit() && b != b'.' && b != b'-' {
//                         is_numeric = false;
//                     }
//                     if write_idx != *cursor {
//                         src[write_idx] = b;
//                     }
//                     write_idx += 1;
//                     *cursor += 1;
//                 }
//             }
//         }
//
//         if is_numeric && (write_idx == word_start || is_edge_case_not_number(&src[word_start..write_idx])) {
//             is_numeric = false;
//         }
//         write_idx
//     };
//
//     Some((word_start, word_end, is_numeric))
// }

fn get_next_word(src: &mut [u8], cursor: &mut usize) -> Option<(usize, usize, bool)> {
    let len = src.len();

    // ---------------------------------------------------------
    // PHASE 1: FIND WORD START
    // Fast-forward through delimiters and trash bytes
    // ---------------------------------------------------------
    let mut cur = *cursor; // we don't use it directly from here, so we don't have to
                              // dereference and stuff
    let word_start = 'finder: loop {
        // I guess it's cleaner this way. Ugh.
        if cur >= len {
            return None;
        }
        // Direct table lookup to skip jump tables/branches
        match GET_CHAR_TYPE[src[*cursor] as usize] {
            CharClass::Other | CharClass::StillNumber => cur += 1,
            CharClass::CCPrefix => cur += 2,
            CharClass::Byte3 => cur += 3,
            CharClass::Byte4 => cur += 4,
            // Valid start: Lower, Upper, Number, C3Prefix
            _ => break 'finder cur,
        }
    };

    let mut write_idx = word_start;
    // ---------------------------------------------------------
    // PHASE 2: NUMERIC LOOP (Optional)
    // Only enters if word starts with a number.
    // ---------------------------------------------------------
    if let CharClass::Number = GET_CHAR_TYPE[src[word_start] as usize] {
        while write_idx < len {
            let b = src[write_idx];
            match GET_CHAR_TYPE[b as usize] {
                // Pure Numeric Characters
                CharClass::Number | CharClass::StillNumber => write_idx += 1,
                // Delimiters -> End of valid Number
                class => {
                    let skip = match class {
                        CharClass::Byte3 => 3,
                        CharClass::Byte4 => 4,
                        CharClass::CCPrefix => 2,
                        _ => 0, // Other/Upper/Lower: don't skip, next call handles them
                    };
                    *cursor = write_idx + skip;
                    return Some((word_start, write_idx, true)); // write_idx is non-inclusive
                }
            }
        }

        // If we reached EOF in numeric mode
        if write_idx >= len {
            *cursor = write_idx;
            return Some((word_start, write_idx, true));
        }
    }

    // ---------------------------------------------------------
    // PHASE 3: SCAN & NORMALIZE
    // ---------------------------------------------------------

    while write_idx < len {
        let b = src[write_idx];

        match GET_CHAR_TYPE[b as usize] {
            CharClass::Lower => {
                write_idx += 1;
            }
            CharClass::Upper => {
                src[write_idx] = b | 0x20;
                write_idx += 1;
            }
            CharClass::Number => {
                // Fix this part. We're supposed to break and return here
                write_idx += 1;
                // is_numeric status is preserved (starts true -> stays true)
            }
            CharClass::C3Prefix => {
                // Handle Albanian NFC (Ë/Ç)
                if write_idx + 1 < len {
                    let next = src[*cursor + 1];
                    src[write_idx] = 0xC3;
                    // Normalize Upper Albanian to Lower:
                    // 0x8B(Ë)->0xAB(ë), 0x87(Ç)->0xA7(ç)
                    match next {
                        0x8B | 0x87 | 0xAB | 0xA7 => src[write_idx + 1] = next | 0x20,
                        _ => src[write_idx + 1] = next,
                    }
                    write_idx += 2;
                } else {
                    write_idx += 1; // Broken UTF-8 at EOF
                    break;
                }
            }
            CharClass::CCPrefix => {
                // BACKTRACKING NFD FIX
                // We hit a combining char (0xCC). Check if we can merge it with the PREVIOUS char.
                if write_idx + 1 < len {
                    let comb = src[*cursor + 1];

                    // Safety: We can only merge if we actually wrote a char previously in this word
                    if write_idx > word_start {
                        let prev = src[write_idx - 1];

                        // Check for 'e' + diaeresis (0x88) OR 'c' + cedilla (0xA7)
                        // Note: prev is already lowercased by previous iterations
                        if prev == b'e' && comb == 0x88 {
                            // Merge: e (1 byte) + CC 88 (2 bytes) -> ë (2 bytes: C3 AB)
                            // We overwrite the 'e' at [write_idx-1]
                            src[write_idx - 1] = 0xC3;
                            src[write_idx] = 0xAB;
                            write_idx += 1; // We added net +1 byte length (1 -> 2)
                        } else if prev == b'c' && comb == 0xA7 {
                            // Merge: c + CC A7 -> ç (C3 A7)
                            src[write_idx - 1] = 0xC3;
                            src[write_idx] = 0xA7;
                            write_idx += 1;
                        }
                        // Else: It's a "pointless" individual diacritic.
                        // We do nothing to write_idx (effectively stripping it).
                    }
                    write_idx += 2; // Always consume the 2-byte combining char
                } else {
                    write_idx += 2;
                    break;
                }
            }
            // Delimiters (Other, Byte3, Byte4, StillNumber)
            _ => break,
        }
    }
    *cursor = write_idx;
    Some((word_start, write_idx, false))
}
