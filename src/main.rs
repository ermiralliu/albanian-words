#![feature(portable_simd)]
use std::io::{BufRead, Write};
use std::simd::Simd;

const SIMD_BYTESIZE: usize = 32;

const _SIMD_BITSIZE: usize = SIMD_BYTESIZE * 8;

type SimdHere = Simd<u8, SIMD_BYTESIZE>;
// type ProcessWordFn = fn(&[u8]) -> ();
// use std::sync::atomic::{AtomicU64, Ordering};
// pub static CALL_COUNT_1: AtomicU64 = AtomicU64::new(0);
// pub static CALL_COUNT_2: AtomicU64 = AtomicU64::new(0);
// pub static CALL_COUNT_3: AtomicU64 = AtomicU64::new(0);
// pub static CALL_COUNT_4: AtomicU64 = AtomicU64::new(0);
// pub static CALL_COUNT_5: AtomicU64 = AtomicU64::new(0);

// pub mod alb_parser;
pub mod alb_parser_imprv;
pub mod bitset;
pub mod categories;
pub mod file_readers;
pub mod properties;
pub mod stop_words;

use std::{fs::File, sync::Mutex, thread, time::Instant, vec};

use file_readers::seq_read;
use fst::raw::Fst;
use properties::Properties;
use std::env;

use crate::bitset::process_streaming_new;
use crate::categories::{get_category_id, get_id_fuzzy};
// use stop_words::STOP_WORDS;

const DEFAULT_VEC_CAPACITY: usize = 256 * 1024;
pub const FST_DATA: &'static [u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/", "dictionary.bin"));
pub const FST_CATEGORIES: &'static [u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/", "categories.bin"));

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    let sr_categories = seq_read::SequentialFileReader::try_new(&config.category_file, config.category_list_boundary)?;
    // let mut count = 0;
    // let set: HashSet<&[u8]> = STOP_WORDS.iter().copied().map(|word| word.as_bytes()).collect();

    // let reg = u64x4::from_array(ARR);

    // These live on the stack of main
    let shared_reader = Mutex::new(sr);
    let shared_reader_categories = Mutex::new(sr_categories);
    // 'set' and 'parser' can just be regular references
    let core_count = num_cpus::get_physical();
    // let core_count = 1;

    let fst = Fst::new(FST_DATA).expect("The vocabulary has not been built successfully");
    let fst_categories =
        Fst::new(FST_CATEGORIES).expect("Categories State Machine hasn't been built successfully");
    let final_tokens: (Vec<Vec<u16>>, Vec<Vec<u16>>) = thread::scope(|s| {
        let mut handles = vec![];
        let scoped_fst = &fst;
        let scoped_fst_categories = &fst_categories;

        for _ in 0..core_count {
            // We borrow from the outer scope
            let r = &shared_reader;
            let r_categories = &shared_reader_categories;
            // let stop_words = &set;

            let h = s.spawn(move || {
                let mut local_buf = Vec::with_capacity(DEFAULT_VEC_CAPACITY);
                let mut local_category_buf = Vec::with_capacity(64);
                let mut thread_results = Vec::with_capacity(128);
                let mut thread_category_results: Vec<Vec<u16>> = Vec::with_capacity(8);
                let local_fst_ref = scoped_fst;

                let mut stack = Vec::new();
                let mut category_stack = Vec::new();

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
                    process_streaming_new(&mut local_buf, &mut |word| {
                        if
                        /* stop_words.contains(word) || */
                        word.len() < 2 || word.len() >= 32 {
                            return;
                        }
                        if word[0].is_ascii_digit() {
                            article_tokens.push(1);
                            return;
                        }
                        // let id = local_parser.single_verb_to_base_new(word);
                        let id = alb_parser_imprv::single_verb_to_base_ultra(local_fst_ref, word, &mut stack);
                        if let Some(id_num) = id {
                            article_tokens.push(id_num + 2); // 0 is `not found` and 1 is number for now -> this should be documented more clearly 
                        } else {
                            article_tokens.push(0);
                        }
                        #[cfg(debug_assertions)]
                        {
                            let printable_word = unsafe { std::str::from_utf8_unchecked(word) };
                            let Some(id) = id else { return };
                            let Some(val) = local_fst_ref.get_key(id as u64 - 1) else { return };
                            let value = std::str::from_utf8(val.as_slice()).unwrap();

                            println!("Word: {}, Tokenization: {}", printable_word, value);
                        }
                    });

                    local_category_buf.clear();
                    {
                        let mut reader = r_categories.lock().unwrap();
                        if !reader.read_into(&mut local_category_buf) {
                            break;
                        }
                    }

                    if !article_tokens.is_empty() {
                        thread_results.push(article_tokens);
                    } else {
                        continue;
                    }
                    // article_categories
                    thread_category_results.push(
                        local_category_buf
                            .as_slice()
                            .split(|x| *x == config.category_entry_separator)
                            .map(|cate| get_category_id(scoped_fst_categories, cate, &mut category_stack))
                            .flatten()
                            .collect(),
                    );
                    // for cat in local_category_buf.as_slice().split(|x| *x == config.category_entry_separator) {
                    //     if let Some(cate) = get_id_fuzzy(scoped_fst_categories, cat){
                    //        thread_category_results.push(cate);
                    //     }
                    // }
                }
                (thread_results, thread_category_results)
            });
            handles.push(h);
        }

        // Join is automatic at the end of the scope, but we collect results here
        // handles.into_iter().flat_map(|h| h.join().unwrap()).collect()
        // handles.into_iter().flat_map(|h| h.join().unwrap()).unzip()
        // let (nested_results, nested_categories): (Vec<Vec<Vec<u16>>>, Vec<Vec<Vec<u16>>>) = handles
        //     .into_iter()
        //     .map(|h| h.join().unwrap()) // Map to the tuple
        //     .unzip(); // Split into two vectors of vectors

        // // Flatten the nesting created by the threads
        // let final_results: Vec<Vec<u16>> = nested_results.into_iter().flatten().collect();
        // let final_categories: Vec<Vec<u16>> = nested_categories.into_iter().flatten().collect();
        handles
            .into_iter()
            .flat_map(|h| {
                let (tokens, cats) = h.join().unwrap();
                // This is the "magic" step you were missing:
                // Convert ([T], [C]) -> Iterator<(T, C)>
                tokens.into_iter().zip(cats)
            })
            .unzip() // Now collects Iterator<(T, C)> -> ([T], [C]))
        // (final_results, final_categories)
    });

    let end = Instant::now();
    let final_val = &final_tokens.0[final_tokens.0.len() - 1];
    let final_cat = &final_tokens.1[final_tokens.1.len() -1 ];
    println!("{:?}", final_val);
    for el in final_val.iter().map(|x,| {
        if *x == 0 {
            return Some(Vec::from(b"Not found"));
        } else if *x == 1 {
            return Some(Vec::from(b"Number"));
        }
        return fst.get_key(*x as u64 - 2);
    }) {
        if let Some(val) = el {
            print!("{:?},", std::str::from_utf8(val.as_slice()).unwrap());
        }
    }
    for el in final_cat.iter().map(|x,| {
        return fst_categories.get_key(*x as u64);
    }) {
        if let Some(val) = el {
            print!("{:?},", std::str::from_utf8(val.as_slice()).unwrap());
        }
    }
    println!("Time passed: {:?}", (end - start));
    // println!("{:?}, {:?}, {:?}, {:?}, {:?}", CALL_COUNT_1, CALL_COUNT_2, CALL_COUNT_3, CALL_COUNT_4, CALL_COUNT_5);
    save_to_file(final_tokens.0, "out/albanian_test.rkyv")?;
    save_to_file(final_tokens.1, "out/albanian_test_categories.rkyv")?;
    Ok(())
}

use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[rkyv(
    // This will generate a PartialEq impl between archived and normal types
    compare(PartialEq),
    // bytecheck can be used to validate your data if you want
    derive(Debug),
)]
struct Data {
    matrix: Vec<Vec<u16>>,
}

// Efficiently save to disk
pub fn save_to_file(matrix: Vec<Vec<u16>>, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Serialize to bytes
    let data = Data { matrix };
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&data)?;

    // Write to file
    let mut file = File::create(path)?;
    file.write_all(&bytes)?;

    println!("Data written successfully!");
    Ok(())
}
