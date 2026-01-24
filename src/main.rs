#![feature(portable_simd)]

pub mod alb_parser;
pub mod bitset;
pub mod file_readers;
pub mod properties;
pub mod stop_words;

use std::{
    collections::{HashMap, HashSet}, simd::u64x4, sync::Mutex, thread, time::Instant, vec
};

use alb_parser::AlbanianParser;
use bitset::{get_first_pass_type, get_second_pass_type, FirstPassType};
use file_readers::seq_read;
use properties::Properties;
use std::env;
use stop_words::STOP_WORDS;

use crate::bitset::ARR;

// #[derive(Copy, Clone, Debug, PartialEq)]
// #[repr(u8)]
// pub enum CharClass {
//     // I had forgotten I could do this and that it's the best possible way of handling such cases.
//     // Delimiter = 0, // Space, punctuation, etc. // Why would i handle these differently?
//     // Ordering these by probability but that probably doesn't matter here ngl
//     Lower,    // a-z
//     Upper,    // A-Z
//     Other,    // Mostly whitespaces and punctuation and other stuff
//     Number,   // 0-9
//     C3Prefix, // 0xC3 (ë, ç)
//     CCPrefix, // 0xCC (Combining marks)
//     Byte3,    // We can instantly skip 3 bytes for these
//     Byte4,    // We can skip 4 for these
//     StillNumber, // If is_number, these are just delimiters
//               // E2Prefix = 6,  // 0xE2 (Smart quotes)
//               // Foreign = 7,   // Everything else (ö, ü, Chinese, etc.)
// } // I'm giving up on the hyphen

// const FIRST_PASS: [bool; 256] = {
//     let mut init = [false; 256];
//     let mut i: u8 = 0;
//
//     loop {
//         if matches!(i, b'0'..=b'9' | b'A'..= b'Z'| b'a'..=b'z'| 0xC3) {
//             init[i as usize] = true;
//         }
//         if i == 255 {
//             break;
//         }
//         i += 1;
//     }
//     init
// };

// const GET_CHAR_TYPE: [CharClass; 256] = {
//     let mut init = [CharClass::Other; 256];
//     let mut i: u8 = 0;
//
//     loop {
//         match i {
//             b'0'..=b'9' => init[i as usize] = CharClass::Number,
//             b'A'..=b'Z' => init[i as usize] = CharClass::Upper,
//             b'a'..=b'z' => init[i as usize] = CharClass::Lower,
//             0xE0..=0xEF => init[i as usize] = CharClass::Byte3,
//
//             // THE 4-BYTE ZONE (11110xxx)
//             0xF0..=0xF7 => init[i as usize] = CharClass::Byte4,
//             _ => {}
//         }
//         if i == 255 {
//             break;
//         }
//         i += 1;
//     }
//     init[b'.' as usize] = CharClass::StillNumber;
//     init[b',' as usize] = CharClass::StillNumber;
//     init[b'\'' as usize] = CharClass::StillNumber;
//     // init[b'-' as usize] = CharClass::Minus;
//     init[0xC3] = CharClass::C3Prefix; // To handle extended latin, to invalidate or get e and c
//     init[0xCC] = CharClass::CCPrefix; // Individual diaeresis 0x88, cedilla 0xA7
//     // init[0xE2] = true;
//     init
// };

const DEFAULT_VEC_CAPACITY: usize = 256 * 1024;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Hello, world!");
    // let verbs = vec!["punojmë", "punuam", "shkruar", "lexuar", "vendosur"];
    let vocab_vector: Vec<&[u8]> = vec![b"punoj", b"shkruaj", b"lexoj", b"vendos"]; // this is just for tests,
    let mut map = HashMap::new();
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
    let set: HashSet<&[u8]> = STOP_WORDS.iter().copied().map(|word| word.as_bytes()).collect();

    let reg = u64x4::from_array(ARR);

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

                    let len = local_buf.len();
                    if len == 0 {
                        continue;
                    }

                    let mut iterator = &mut local_buf[0] as *mut u8;
                    let end = unsafe { iterator.add(len) };

                    // while (let res = get_next_word(&mut iterator, end)) != None {
                    while iterator != end {
                        let id = match get_next_word(&mut iterator, end, reg) {
                            WordType::None => break,
                            WordType::ValidWord(items) => {
                                if stop_words.contains(items) {
                                    continue;
                                }
                                #[cfg(debug_assertions)]
                                {
                                    dbg!(unsafe { std::str::from_utf8_unchecked(items) });
                                }
                                local_parser.single_verb_to_base(items).unwrap_or(0)
                            }
                            WordType::Number(items) => {
                                #[cfg(debug_assertions)]
                                {
                                    dbg!(unsafe { std::str::from_utf8_unchecked(items) });
                                }
                                continue; // this will obviously be fixed
                            }
                            WordType::LikelyForeign => 0,
                        };
                        article_tokens.push(id);
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

#[inline(always)]
fn scan_while<F>(mut ptr: *mut u8, end: *mut u8, mut predicate: F) -> *mut u8
where
    F: FnMut(u8) -> bool,
{
    while ptr < end && predicate(unsafe { *ptr }) {
        ptr = unsafe { ptr.add(1) };
    }
    ptr
}

#[inline(always)]
fn get_next_word<'a>(itr_ref: &'a mut *mut u8, end: *mut u8, reg: u64x4) -> WordType<'a> {
    // ---------------------------------------------------------
    // PHASE 1: FIND WORD START
    // ---------------------------------------------------------
    // let word_start = iterator // the return is actually a ptr/reference, so we're actually good
    //     .find(|&b| BITSET.contains(*b as usize))?; // position is after skip, so it's relative, we need to sum with the initial part
    // let size_hint = iterator.len();
    // let start_ptr = word_start as *const u8;
    let mut itr = *itr_ref;
    // if itr == end {
    //     return None;
    //     // kur behet return None del nga loop, kshu qe ska nevoje te incr itr
    // } // Reached end of input

    let (token_type, start_ptr) = loop {
        let ch = unsafe { *itr };
        let res = get_first_pass_type(ch, reg);

        if res != FirstPassType::Skip {
            break (res, itr);
        }

        itr = unsafe { itr.add(1) };
        if itr == end {
            return WordType::None;
            // kur behet return None del nga loop, kshu qe ska nevoje te incr itr
        } // Reached end of input
    };

    // ---------------------------------------------------------
    // New Code
    // ---------------------------------------------------------
    let mut is_foreign = false;
    match token_type {
        FirstPassType::Skip => unreachable!("We have already returned if it was a Skip"),
        FirstPassType::Number => {
            // PHASE 2: SCAN NUMBERS
            // Equivalent to your old next_if(Number | StillNumber)
            itr = scan_while(itr, end, |b| get_first_pass_type(b, reg) == FirstPassType::Number);

            // Update the caller's iterator reference
            *itr_ref = itr;

            let len = unsafe { itr.offset_from(start_ptr) as usize };
            let sl = unsafe { std::slice::from_raw_parts(start_ptr, len) };
            return WordType::Number(sl);
        }
        FirstPassType::Letter => {
            let move_one = unsafe { (*itr == 0xC3) as usize & (itr.add(1) != end) as usize };
            itr = unsafe { itr.add(move_one) };
            unsafe { *itr |= 0x20 };
            itr = unsafe { itr.add(1) }; // we check later that it's not equal to end so it's okay

            // itr was already incremented by 1 after the loop to point to the next char
            while itr < end {
                let b = unsafe { *itr };
                // terrible idea to name it FIRST_PASS
                match get_second_pass_type(b, reg) {
                    bitset::SecondPassType::END => break,
                    bitset::SecondPassType::Letter => {
                        unsafe { *itr |= 0x20 };
                    }
                    bitset::SecondPassType::XC3 => {
                        unsafe {
                            let next_ptr = itr.add(1);
                            if next_ptr == end {
                                break;
                            }
                            *next_ptr |= 0x20;
                            itr = next_ptr;
                            if !matches!(*next_ptr, 0xab | 0xa7) {
                                is_foreign = true;
                            }
                        };
                    }
                    bitset::SecondPassType::XCC => {
                        break;
                        unsafe {
                            let en = if itr.add(10) < end { itr.add(10) } else { end };
                            let offset = en.offset_from(itr);
                            let stri = std::slice::from_raw_parts(itr.sub(1), offset as usize);
                            let sta = std::str::from_utf8_unchecked(stri);
                            println!("String: {}, length: {}", sta, sta.len());
                        }
                        panic!("It wasn't supposed to happen this way. Sorry.");
                    }
                }
                unsafe { itr = itr.add(1) };
            }
            //     match GET_CHAR_TYPE[b as usize] {
            //         CharClass::CCPrefix => unsafe {
            //             // let next_ptr = itr.add(1);
            //             itr = itr.add(1);
            //             // if next_ptr < end {
            //             //     let comb = *next_ptr;
            //             //     // Safety: write_ptr is always >= start_ptr.add(1) here
            //             //     let prev_ptr = write_ptr.sub(1);
            //             //     let prev = *prev_ptr;
            //             //
            //             //     if prev == b'e' && comb == 0x88 {
            //             //         *prev_ptr = 0xc3;
            //             //         *write_ptr = 0xab;
            //             //         write_ptr = write_ptr.add(1);
            //             //     } else if prev == b'c' && comb == 0xa7 {
            //             //         *prev_ptr = 0xc3;
            //             //         *write_ptr = 0xa7;
            //             //         write_ptr = write_ptr.add(1);
            //             //     } else {
            //             //         // No match, just consume (or TODO: handle 3-byte normalization)
            //             //     }
            //             //     itr = itr.add(2);
            //             // } else {
            //             //     itr = itr.add(1);
            //             //     break;
            //             // }
            //         },
            //         _ => break,
            //     }
            // }
            *itr_ref = itr;
            if is_foreign {
                return WordType::LikelyForeign;
            }
            // The part below would normally use write_ptr, but I guess we return it normally from
            // here + a flag.
            let final_len = unsafe { itr.offset_from(start_ptr) as usize };
            let sl = unsafe { std::slice::from_raw_parts(start_ptr, final_len) };
            WordType::ValidWord(sl)
        }
    }
}

enum WordType<'a> {
    None,
    ValidWord(&'a [u8]),
    Number(&'a [u8]),
    LikelyForeign,
}
