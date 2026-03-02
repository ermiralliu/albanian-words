#![feature(portable_simd)]
use std::fmt::Debug;
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
pub mod map_category;
pub mod properties;
pub mod stop_words;

use std::{fs::File, sync::Mutex, thread, time::Instant, vec};

use file_readers::seq_read;
use fst::raw::Fst;
use itertools::Itertools;
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
    // let mut count = 0;
    // let set: HashSet<&[u8]> = STOP_WORDS.iter().copied().map(|word| word.as_bytes()).collect();

    // let reg = u64x4::from_array(ARR);

    // Shared variables
    // These live on the stack of main
    let sr = seq_read::SequentialFileReader::try_new(&config.article_file, config.article_separator)?;
    let sr_categories = seq_read::SequentialFileReader::try_new(&config.category_file, config.category_list_boundary)?;
    // The threads will modify the inner pointer that keeps track of the position in the file, so
    // this needs a mutex to avoid simultaneous reads/writes.
    let shared_reader = Mutex::new((sr, sr_categories));
    
    let core_count = num_cpus::get_physical();
    let fst = Fst::new(FST_DATA).expect("The vocabulary has not been built successfully");
    let fst_categories = Fst::new(FST_CATEGORIES).expect("Categories State Machine hasn't been built successfully");

    let final_tokens: (Vec<Vec<u16>>, Vec<Vec<u8>>) = thread::scope(|s| {
        let mut handles = vec![];
        let scoped_fst = &fst;
        let scoped_fst_categories = &fst_categories;

        for _ in 0..core_count {
            // We borrow from the outer scope
            let r = &shared_reader;
            // let r_categories = &shared_reader_categories;
            // let stop_words = &set;

            let h = s.spawn(move || {
                let mut local_buf = Vec::with_capacity(DEFAULT_VEC_CAPACITY);
                let mut local_category_buf = Vec::with_capacity(64);
                let mut thread_results = Vec::with_capacity(128);
                let mut thread_category_results: Vec<Vec<u8>> = Vec::with_capacity(8);
                let local_fst_ref = scoped_fst;

                let mut stack = Vec::new();
                let mut category_stack = Vec::new();

                loop {
                    local_buf.clear();
                    local_category_buf.clear();
                    // Lock the reader just to fill the buffer
                    {
                        let mut reader = r.lock().unwrap();

                        if !reader.0.read_into(&mut local_buf) || !reader.1.read_into(&mut local_category_buf) {
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
                            let Some(id) = id else {
                                println!("Word without tokenization: {}", printable_word);
                                return;
                            };
                            let Some(val) = local_fst_ref.get_key(id as u64) else { return };
                            let value = std::str::from_utf8(val.as_slice()).unwrap();
                            if value.len().abs_diff(printable_word.len()) > 3 {
                                println!("Word: {}, Tokenization: {}", printable_word, value);
                            }
                        }
                    });

                    if article_tokens.is_empty() {
                        continue;
                    }

                    let categories_result: Vec<u8> = local_category_buf
                        .as_mut_slice()
                        .split(|x| *x == config.category_entry_separator)
                        .map(|cate| {
                            let str_cause_this_shit_is_annyoing = unsafe { std::str::from_utf8_unchecked(cate) };
                            let cat_trimmed =
                                str_cause_this_shit_is_annyoing.trim_matches(|x: char| !x.is_alphanumeric());
                            if let Some(bad_category_id) =
                                get_category_id(scoped_fst_categories, cat_trimmed.as_bytes(), &mut category_stack)
                            {
                                map_category::map_category_id(bad_category_id) as u8
                            } else {
                                0
                            }
                        })
                        .unique()
                        .collect();
                    // article_categories
                    #[cfg(debug_assertions)]
                    {
                        let cat_str = unsafe { std::str::from_utf8_unchecked(&local_category_buf) };
                        let results: Vec<Vec<u8>> = categories_result
                            .iter()
                            .map(|x| scoped_fst_categories.get_key(*x as u64))
                            .flatten()
                            .collect();
                        let res_str: Vec<&str> = results
                            .iter()
                            .map(|x| unsafe { std::str::from_utf8_unchecked(x) })
                            .collect();
                        println!("Initial categories: {}", cat_str);
                        println!("Categories result: {:?}, as arr: {:?}", res_str, categories_result);
                    }
                    if categories_result.len() > 0 {
                        thread_category_results.push(categories_result);
                        thread_results.push(article_tokens);
                    }
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
    let final_cat = &final_tokens.1[final_tokens.1.len() - 1];
    println!("{:?}", final_val);
    for el in final_val.iter().map(|x| {
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
    for el in final_cat.iter().map(|x| {
        return fst_categories.get_key(*x as u64);
    }) {
        if let Some(val) = el {
            print!("{:?},", std::str::from_utf8(val.as_slice()).unwrap());
        }
    }
    println!("Time passed: {:?}", (end - start));
    // println!("{:?}, {:?}, {:?}, {:?}, {:?}", CALL_COUNT_1, CALL_COUNT_2, CALL_COUNT_3, CALL_COUNT_4, CALL_COUNT_5);
    save_to_file_u16(final_tokens.0, &config.articles_file_out)?; // "out/albanian_test.rkyv"
    save_to_file_u8(final_tokens.1, &config.category_file_out)?; // "out/albanian_test_categories.rkyv"
    Ok(())
}

use rkyv::{Archive, Deserialize, Serialize};
// 1. Define a trait

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[rkyv(
    // This will generate a PartialEq impl between archived and normal types
    compare(PartialEq),
    // bytecheck can be used to validate your data if you want
    derive(Debug),
)]
struct DataU16{
    matrix: Vec<Vec<u16>>,
}

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[rkyv(
    // This will generate a PartialEq impl between archived and normal types
    compare(PartialEq),
    // bytecheck can be used to validate your data if you want
    derive(Debug),
)]
struct DataU8{
    matrix: Vec<Vec<u8>>,
}

pub fn save_to_file_u16(matrix: Vec<Vec<u16>>, path: &str) -> Result<(), Box<dyn std::error::Error>>
{
    
    // Serialize to bytes
    let data = DataU16 { matrix };
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&data)?;

    // Write to file
    let mut file = File::create(path)?;
    file.write_all(&bytes)?;

    println!("Data written successfully!");
    Ok(())
}

pub fn save_to_file_u8(matrix: Vec<Vec<u8>>, path: &str) -> Result<(), Box<dyn std::error::Error>>
{
    
    // Serialize to bytes
    let data = DataU8 { matrix };
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&data)?;

    // Write to file
    let mut file = File::create(path)?;
    file.write_all(&bytes)?;

    println!("Data written successfully!");
    Ok(())
}
// use rkyv::ser::allocator::ArenaHandle;
// use rkyv::util::AlignedVec;
// use rkyv::rancor::Error;

// pub fn save_to_file<T>(matrix: Vec<Vec<T>>, path: &str) -> Result<(), Box<dyn std::error::Error>>
// where
//     T: std::fmt::Debug + Archive,
//     for<'a> T: rkyv::Serialize<rkyv::api::high::HighSerializer<AlignedVec, ArenaHandle<'a>, Error>>,
// {
//     let data = Data { matrix };
//     let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&data)?;

//     let mut file = File::create(path)?;
//     file.write_all(&bytes)?;

//     println!("Data written successfully!");
//     Ok(())
// }

// Efficiently save to disk
// #[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
// #[rkyv(compare(PartialEq))]
// struct Data<T: std::fmt::Debug + Archive> {
//     matrix: Vec<Vec<T>>,
// }

// impl<T> std::fmt::Debug for ArchivedData<T>
// where
//     T: Archive + std::fmt::Debug,
//     <T as Archive>::Archived: std::fmt::Debug,
// {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.debug_struct("ArchivedData")
//             .field("matrix", &self.matrix)
//             .finish()
//     }
// }

// pub fn save_to_file<T>(matrix: Vec<Vec<T>>, path: &str) -> Result<(), Box<dyn std::error::Error>>
// where
//     T: std::fmt::Debug + Archive,
//     <T as Archive>::Archived: std::fmt::Debug,
//     for<'a> T: rkyv::Serialize<rkyv::api::high::HighSerializer<rkyv::util::AlignedVec, rkyv::ser::allocator::ArenaHandle<'a>, rkyv::rancor::Error>>,
//     for<'a> Vec<T>: rkyv::Serialize<rkyv::api::high::HighSerializer<rkyv::util::AlignedVec, rkyv::ser::allocator::ArenaHandle<'a>, rkyv::rancor::Error>>,
// {
//     let data = Data { matrix };
//     let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&data)?;

//     let mut file = File::create(path)?;
//     file.write_all(&bytes)?;

//     println!("Data written successfully!");
//     Ok(())
// }
