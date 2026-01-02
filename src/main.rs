pub mod alb_parser;
pub mod file_readers;
use regex::Regex;
use unicode_normalization::UnicodeNormalization;
use std::{collections::HashMap, vec};

use alb_parser::AlbanianParser;
use file_readers::seq_read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Hello, world!");
    let verbs = vec!["punojmë", "punuam", "shkruar", "lexuar", "vendosur"];
    let vocab_vector = vec!["punoj", "shkruaj", "lexoj", "vendos"]; // this is just for tests,
    let mut map = HashMap::new();
    for (i, &word) in vocab_vector.iter().enumerate() {
        map.insert(word, i as u16);
    }
    let mut parser = AlbanianParser::new(&map);
    // for el in parser.verb_to_base(&verbs) {
    //     println!("{}", vocab_vector[el as usize]);
    // }

    let mut sr = seq_read::SequentialFileReader::try_new(
        "/home/ermir/Documents/Diploma/csv_processing/finalized-content/test_content.txt",
        b'\x1E',
    )?;
    let mut token_container = Vec::new();
    let mut sentence_normalization_buffer = String::with_capacity(256*1024);
    let mut sentence_lowercasing_buffer = sentence_normalization_buffer.clone();
    while let Some(line) = sr.read() {
        sentence_normalization_buffer.extend(line.nfc());
        sentence_lowercasing_buffer.extend(sentence_normalization_buffer.chars().flat_map(|ch| ch.to_lowercase()));

        let regex = Regex::new(r"([a-zëç-]+)")?; // Only lowercase since we alr
                                                            // normalized
        // separate each word
        let mut sentence_tokens = Vec::new();
        for mat in regex.find_iter(&sentence_lowercasing_buffer) {
            let mat_str = mat.as_str();
            if let Some(nr) = parser.single_verb_to_base(mat_str) {
                sentence_tokens.push(nr);
            }
        }
        if !sentence_tokens.is_empty() {
            token_container.push(sentence_tokens);
        }

        sentence_normalization_buffer.clear();
        sentence_lowercasing_buffer.clear();
    }
    println!("{:?}", token_container);
    Ok(())
    // let Some(n) = return_some_option() else { return };
}
