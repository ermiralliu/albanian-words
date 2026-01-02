pub mod alb_parser;
pub mod file_readers;
use regex::Regex;
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
    for el in parser.verb_to_base(&verbs) {
        println!("{}", vocab_vector[el as usize]);
    }

    let mut sr = seq_read::SequentialFileReader::try_new(
        "/home/ermir/Documents/Diploma/csv_processing/finalized-content/test_content.txt",
        b'\x1E',
    )?;
    while let Some(line) = sr.read() {
        // separate each word
        let regex = Regex::new(r"([a-zA-Zëç-]+)")?; // shto ë dhe ç te madhe
        for mat in regex.find_iter(line) {
            let mat_str = mat.as_str();
        }
    }
    Ok(())
    // let Some(n) = return_some_option() else { return };
}
