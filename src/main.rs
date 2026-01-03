pub mod alb_parser;
pub mod file_readers;
use regex::Regex;
use std::{
    collections::{HashMap, HashSet},
    fs::read,
    time::Instant,
    vec,
};
use unicode_normalization::UnicodeNormalization;

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
];

// const WORD_DELIMITERS: &[char] = &[
//     '.', ',', '/', '\\', ' ', '\n', '\t', '\"', '\'', ':', ';', '!', '?', '(', ')', '[', ']', '\r', '\x1E', '“', '”',
//     '–', // fatkeqesisht, data set-i perdor keto quot-et e cuditshme, which
//         // kinda ruins stuff
//         // so using this non regex method is not that secure maybe?
// ];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Hello, world!");
    // let verbs = vec!["punojmë", "punuam", "shkruar", "lexuar", "vendosur"];
    let vocab_vector: Vec<&[u8]> = vec![b"punoj", b"shkruaj", b"lexoj", b"vendos"]; // this is just for tests,
    let mut map = HashMap::new();
    for (i, &word) in vocab_vector.iter().enumerate() {
        map.insert(word, i as u16);
    }
    let mut parser = AlbanianParser::new(&map);
    // for el in parser.verb_to_base(&verbs) {
    //     println!("{}", vocab_vector[el as usize]);
    // }
    let start = Instant::now();

    let mut sr = seq_read::SequentialFileReader::try_new(
        "/home/ermir/Documents/Diploma/csv_processing/finalized-content/test_content.txt",
        b'\x1E',
    )?;
    let mut token_container = Vec::new();
    // let mut sentence_normalization_buffer = String::with_capacity(256 * 1024);
    let mut article_buffer: Vec<u8> = Vec::with_capacity(256 * 1024);
    let mut count = 0;
    let set: HashSet<&[u8]> = STOP_WORDS.iter().copied().map(|word| word.as_bytes()).collect();
    // let regex: Regex = Regex::new(r"([a-zëç-]+)")?; // Only lowercase since we alr
    while sr.read_into(&mut article_buffer) {
        count += 1;
        // sentence_lowercasing_buffer.extend(line.nfc());

        // sentence_lowercasing_buffer.extend(sentence_normalization_buffer.chars().flat_map(|ch| ch.to_lowercase()));
        albanian_clean_inplace(&mut article_buffer);
        #[cfg(debug_assertions)]
        {
            println!("Word: {:?}", unsafe { str::from_utf8_unchecked(&article_buffer[..]) },);
        }

        // normalized
        // separate each word
        let mut sentence_tokens = Vec::new();
        let clean_words = article_buffer
            .split(|x| WORD_DELIMITERS.contains(x))
            .filter(|s| s.len() > 1 && !set.contains(s));
        for mat_str in clean_words {
            #[cfg(debug_assertions)]
            {
                // println!(
                //     "Word: {:?}, len: {}",
                //     unsafe { str::from_utf8_unchecked(mat_str) },
                //     mat_str.len()
                // );
            }
            // if mat_str.len() == 570 {
            //     println!("Line: {}", unsafe{ std::str::from_utf8_unchecked(&article_buffer[..])});
            //     println!("{}", unsafe { std::str::from_utf8_unchecked(mat_str) });
            //     return Ok(());
            // }
            

            // if mat_str.parse::<f64>().is_ok() { // need to find a new way to handle numbers
            if let Some(nr) = parser.single_verb_to_base(mat_str) {
                sentence_tokens.push(nr);
            } else {
                sentence_tokens.push(0);
            }
        }
        #[cfg(debug_assertions)]
        {
            // println!("Line: {}\nTokenized:{:?}", line, sentence_tokens);
        }
        if !sentence_tokens.is_empty() {
            token_container.push(sentence_tokens);
        }

        if count >= 100000 {
            break;
        }
        article_buffer.clear();
    }
    let end = Instant::now();
    println!("{:?}", token_container[token_container.len() - 1]);
    println!("Time passed: {:?}", (end - start));
    Ok(())
    // let Some(n) = return_some_option() else { return };
}

pub fn albanian_clean_inplace(bytes: &mut Vec<u8>) {
    let mut read_idx = 0;
    let mut write_idx = 0;
    let len = bytes.len();

    while read_idx < len {
        match bytes[read_idx] {
            // 1. Handle Decomposed 'e' or 'E' + 'diaeresis' (NFD -> NFC)
            // Shrinks 3 bytes into 2 bytes
            0x65 | 0x45 if read_idx + 2 < len && bytes[read_idx + 1] == 0xCC && bytes[read_idx + 2] == 0x88 => {
                bytes[write_idx] = 0xC3;
                bytes[write_idx + 1] = 0xAB; // 'ë'
                read_idx += 3;
                write_idx += 2;
            }

            // 2. Handle ASCII Uppercase -> Lowercase
            b'A'..=b'Z' => {
                bytes[write_idx] = bytes[read_idx] + 32;
                read_idx += 1;
                write_idx += 1;
            }

            // 3. Handle Albanian Ç / ç and Ë / ë (UTF-8 lead byte 0xC3)
            0xC3 if read_idx + 1 < len => {
                let next = bytes[read_idx + 1];
                bytes[write_idx] = 0xC3;
                match next {
                    0x8B | 0xAB => bytes[write_idx + 1] = 0xAB, // ë
                    0x87 | 0xA7 => bytes[write_idx + 1] = 0xA7, // ç
                    _ => bytes[write_idx + 1] = next,
                }
                read_idx += 2;
                write_idx += 2;
            }

            // 4. Handle Multi-byte Delimiters (3 bytes: “, ”, –)
            // Replace with spaces to "flatten" for the split() call later
            0xE2 if read_idx + 2 < len && bytes[read_idx + 1] == 0x80 => {
                let third = bytes[read_idx + 2];
                if third == 0x9C || third == 0x9D || third == 0x93 || third == 0x98 || third == 0x99 {
                    // Here are the special single and double quotes and dash that some articles used
                    bytes[write_idx] = b' '; // Flatten to a single space
                    read_idx += 3; // Consume 3 bytes
                    write_idx += 1; // Advance 1 byte in output
                } else {
                    // Not a target delimiter, copy the lead byte
                    bytes[write_idx] = bytes[read_idx];
                    read_idx += 1;
                    write_idx += 1;
                }
            }

            // 5. Everything else
            _ => {
                if read_idx != write_idx {
                    bytes[write_idx] = bytes[read_idx];
                }
                read_idx += 1;
                write_idx += 1;
            }
        }
    }

    // Finalize the length of the vector
    // This is better than fill(b' ') because it makes the buffer shorter
    // so subsequent processing (splitting) has less data to look at.
    unsafe {
        bytes.set_len(write_idx);
    }
}
