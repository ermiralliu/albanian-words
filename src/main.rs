pub mod alb_parser;
pub mod file_readers;
use std::{
    collections::{HashMap, HashSet},
    sync::Mutex,
    thread,
    time::Instant,
    vec,
};

use alb_parser::AlbanianParser;
use file_readers::seq_read;

const STOP_WORDS: &[&str] = &[
    "e",
    "të",
    "i",
    "me",
    "që",
    "në",
    "një",
    "a",
    "për",
    "sh",
    "nga",
    "ka",
    "u",
    "është",
    "dhe",
    "shih",
    "nuk",
    "m",
    "diçka",
    "ose",
    "si",
    "shumë",
    "etj",
    "se",
    "pa",
    "sipas",
    "s",
    "t",
    "dikujt",
    "dikë",
    "mirë",
    "vetë",
    "bëj",
    "ai",
    "vend",
    "prej",
    "ja",
    "duke",
    "tjetër",
    "kur",
    "ia",
    "ku",
    "ta",
    "keq",
    "dy",
    "bën",
    "bërë",
    "bëhet",
    "diçkaje",
    "edhe",
    "madhe",
    "la",
    "sa",
    "gjatë",
    "zakonisht",
    "pas",
    "veta",
    "mbi",
    "disa",
    "iu",
    "mos",
    "ç",
    "para",
    "dikush",
    "gjë",
    "bë",
    "pak",
    "tek",
    "farë",
    "bëri",
    "po",
    "bie",
    "k",
    "do",
    "gjithë",
    "vete",
    "mund",
    "kam",
    "lë",
    "jo",
    "bëje",
    "tij",
    "kanë",
    "ishte",
    "janë",
    "vjen",
    "atë",
    "këtë",
    "nëpër",
    "çdo",
    "na",
    "marrë",
    "merr",
    "mori",
    "rri",
    "deri",
    "b",
    "kishte",
    "mban",
    "përpara",
    "tyre",
    "marr",
    "gjitha",
    "as",
    "vetëm",
    "nën",
    "herë",
    "tjera",
    "tjerët",
    "drejt",
    "qenët",
    "ndonjë",
    "nëse",
    "jap",
    "merret",
    "rreth",
    "lloj",
    "dot",
    "saj",
    "ndër",
    "ndërsa",
    "cila",
    "veten",
    "ma",
    "ndaj",
    "mes",
    "ajo",
    "cilën",
    "por",
    "ndërmjet",
    "prapa",
    "mi",
    "tërë",
    "jam",
    "ashtu",
    "kësaj",
    "tille",
    "bëhem",
    "cilat",
    "kjo",
    "menjëherë",
    "ça",
    "je",
    "aq",
    "aty",
    "pranë",
    "ato",
    "pasur",
    "qenë",
    "cilin",
    "tepër",
    "njëra",
    "tej",
    "krejt",
    "kush",
    "bëjnë",
    "ti",
    "bënë",
    "midis",
    "cili",
    "ende",
    "këto",
    "kemi",
    "siç",
    "kryer",
    "çilit",
    "atij",
    "gjithnjë",
    "andej",
    "sipër",
    "sikur",
    "këtej",
    "cilës",
    "ky",
    "papritur",
    "ua",
    "kryesisht",
    "gjithçka",
    "pasi",
    "kryhet",
    "mjaft",
    "këtij",
    "përbashkët",
    "ata",
    "atje",
    "vazhdimisht",
    "kurrë",
    "tonë",
    "kështu",
    "unë",
    "sapo",
    "rrallë",
    "vetes",
    "ishin",
    "afërt",
    "tjetrën",
    "këtu",
    "çfarë",
    "to",
    "anës",
    "jemi",
    "asaj",
    "secila",
    "kundrejt",
    "këtyre",
    "pse",
    "tilla",
    "mua",
    "nëpërmjet",
    "çilet",
    "ndryshe",
    "kishin",
    "ju",
    "tani",
    "atyre",
    "diç",
    "ynë",
    "kudo",
    "sonë",
    "sepse",
    "çilave",
    "kemi",
    "ty",
    "t",
    "nbsp", // This needs more work though
    "tha",
    "re",
    "the",
    "€",
];

const WORD_DELIMITERS: &[u8] = &[
    b'.', b',', b'/', b'\\', b' ', b'\n', b'\t', b'\"', b'\'', b':', b';', b'!', b'?', b'(', b')', b'[', b']', b'\r',
    0x1E, 0x1F,
];

const WORD_DELIMITER_BITSET: [bool; 256] = {
    let mut init = [false; 256];
    let mut i = 0;
    while i < WORD_DELIMITERS.len() {
        init[WORD_DELIMITERS[i] as usize] = true;
        i += 1;
    }
    init
};

// const WORD_DELIMITERS: &[char] = &[
//     '.', ',', '/', '\\', ' ', '\n', '\t', '\"', '\'', ':', ';', '!', '?', '(', ')', '[', ']', '\r', '\x1E', '“', '”',
//     '–', // fatkeqesisht, data set-i perdor keto quot-et e cuditshme, which
//         // kinda ruins stuff
//         // so using this non regex method is not that secure maybe?
// ];

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

    let final_tokens: Vec<Vec<u16>> = thread::scope(|s| {
        let mut handles = vec![];

        for _ in 0..2 {
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

fn next_word_inplace(src: &mut [u8], cursor: &mut usize) -> Option<(usize, usize, bool)> {
    let len = src.len();

    // 1. Skip leading delimiters (including the 3-byte ones)
    while *cursor < len {
        let b = src[*cursor];

        // Handle ASCII delimiters
        if WORD_DELIMITER_BITSET[b as usize] {
            *cursor += 1;
            continue;
        }

        // Handle 3-byte delimiters at the start
        if b == 0xE2 && *cursor + 2 < len && src[*cursor + 1] == 0x80 {
            let third = src[*cursor + 2];
            if third == 0x9C || third == 0x9D || third == 0x93 || third == 0x98 || third == 0x99 {
                *cursor += 3;
                continue;
            }
        }

        break; // Found a non-delimiter byte
    }

    if *cursor >= len {
        return None;
    }

    let word_start = *cursor;
    let mut write_idx = *cursor;
    let mut is_numeric = true;

    // 2. Scan and transform
    while *cursor < len {
        let b = src[*cursor];

        // ASCII Delimiter check
        if WORD_DELIMITERS.contains(&b) {
            // We do NOT increment cursor here; the next call's "skip" logic handles it
            break;
        }

        match b {
            // Rule 1: NFC Normalization (3 -> 2 bytes)
            0x65 | 0x45 if *cursor + 2 < len && src[*cursor + 1] == 0xCC && src[*cursor + 2] == 0x88 => {
                src[write_idx] = 0xC3;
                src[write_idx + 1] = 0xAB;
                write_idx += 2;
                *cursor += 3;
                is_numeric = false;
            }

            // Rule 2: ASCII Lowercase
            b'A'..=b'Z' => {
                src[write_idx] = b + 32;
                write_idx += 1;
                *cursor += 1;
                is_numeric = false;
            }

            // Rule 3: Albanian Ë/Ç
            0xC3 if *cursor + 1 < len => {
                let next = src[*cursor + 1];
                src[write_idx] = 0xC3;
                match next {
                    0x8B | 0xAB => src[write_idx + 1] = 0xAB,
                    0x87 | 0xA7 => src[write_idx + 1] = 0xA7,
                    _ => src[write_idx + 1] = next,
                }
                write_idx += 2;
                *cursor += 2;
                is_numeric = false;
            }

            // Rule 4: 3-byte Delimiters (The "Stuck" fix)
            0xE2 if *cursor + 2 < len && src[*cursor + 1] == 0x80 => {
                let third = src[*cursor + 2];
                if third == 0x9C || third == 0x9D || third == 0x93 || third == 0x98 || third == 0x99 {
                    // Stop word here. We don't increment cursor; the skip logic above will jump 3.
                    break;
                } else {
                    src[write_idx] = b;
                    write_idx += 1;
                    *cursor += 1;
                    is_numeric = false;
                }
            }

            // Rule 5: Standard scan
            _ => {
                if is_numeric && !b.is_ascii_digit() && b != b'.' && b != b'-' {
                    is_numeric = false;
                }
                if write_idx != *cursor {
                    src[write_idx] = b;
                }
                write_idx += 1;
                *cursor += 1;
            }
        }
    }

    if is_numeric && (write_idx == word_start || is_edge_case_not_number(&src[word_start..write_idx])) {
        is_numeric = false;
    }

    Some((word_start, write_idx, is_numeric))
}

#[inline(always)]
fn is_edge_case_not_number(s: &[u8]) -> bool {
    s == b"." || s == b"-" || s == b".."
}
