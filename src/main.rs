pub mod alb_parser;
pub mod file_readers;
use regex::Regex;
use std::{
    collections::{HashMap, HashSet}, time::Instant, vec
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
    "çilën",
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
    "çilat",
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
    "çilin",
    "tepër",
    "njëra",
    "tej",
    "krejt",
    "kush",
    "bëjnë",
    "ti",
    "bënë",
    "midis",
    "çili",
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
    "çilës",
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
    "nbsp",
    "tha",
    "re",
    "the",
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Hello, world!");
    // let verbs = vec!["punojmë", "punuam", "shkruar", "lexuar", "vendosur"];
    let vocab_vector = vec!["punoj", "shkruaj", "lexoj", "vendos"]; // this is just for tests,
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
    let mut sentence_lowercasing_buffer = String::with_capacity(256 * 1024);
    let mut count = 0;
    let set: HashSet<&str> = STOP_WORDS.iter().copied().collect();
    let regex: Regex = Regex::new(r"([a-zëç-]+)")?; // Only lowercase since we alr
    while let Some(line) = sr.read() {
        // line is actually an entire article. We're not divinding by sentences.
        // Idk why sometimes that is not obvious.
        count += 1;
        sentence_lowercasing_buffer.extend(line.nfc());
        // sentence_lowercasing_buffer.extend(sentence_normalization_buffer.chars().flat_map(|ch| ch.to_lowercase()));
        faster_lowercase(&mut sentence_lowercasing_buffer);

        // normalized
        // separate each word
        let mut sentence_tokens = Vec::new();
        for mat in regex.find_iter(&sentence_lowercasing_buffer) {
            let mat_str = mat.as_str();
            if set.contains(mat_str) {
                continue;
            }
            #[cfg(debug_assertions)]
            {
                // println!("Word: {}", mat_str);
            }
            if let Some(nr) = parser.single_verb_to_base(mat_str) {
                sentence_tokens.push(nr);
            }
            // else {sentence_tokens.push(0)}
        }
        #[cfg(debug_assertions)]
        {
            // println!("Line: {}\nTokenized:{:?}", line, sentence_tokens);
        }
        if !sentence_tokens.is_empty() {
            token_container.push(sentence_tokens);
        }

        // sentence_normalization_buffer.clear();
        sentence_lowercasing_buffer.clear();
        if count >= 1000 {
            break;
        }
    }
    let end = Instant::now();
    println!("{:?}", token_container);
    println!("Time passed: {:?}", (end-start));
    Ok(())
    // let Some(n) = return_some_option() else { return };
}

pub fn faster_lowercase(s: &mut String) { // albanian-specific
    let bytes = unsafe { s.as_mut_vec() };
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            // Standard ASCII uppercase A-Z
            b'A'..=b'Z' => {
                bytes[i] += 32;
                i += 1;
            }
            // UTF-8 for Ë is [C3, 8B], for ë is [C3, AB]
            0xC3 if i + 1 < bytes.len() && bytes[i + 1] == 0x8B => {
                bytes[i + 1] = 0xAB; 
                i += 2;
            }
            // UTF-8 for Ç is [C3, 87], for ç is [C3, A7]
            0xC3 if i + 1 < bytes.len() && bytes[i + 1] == 0x87 => {
                bytes[i + 1] = 0xA7;
                i += 2;
            }
            // Skip everything else (already lowercase or other symbols)
            b => {
                // Determine how many bytes to skip based on UTF-8 lead byte
                i += if b < 0x80 { 1 } else if b < 0xE0 { 2 } else if b < 0xF0 { 3 } else { 4 };
            }
        }
    }
}

