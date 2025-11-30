pub mod alb_parser;
use std::{collections::HashMap, vec};

use alb_parser::AlbanianParser;

fn main() {
    println!("Hello, world!");
    let verbs = vec!["punojmë", "punuam", "shkruar", "lexuar", "vendosur"];
    let vocab_vector = vec!["punoj", "shkruaj", "lexoj", "vendos"]; // this is just for tests,
    let mut map = HashMap::new();
    for (i, &word) in vocab_vector.iter().enumerate() {
        map.insert(word, i as u16);
    }
    let mut parser = AlbanianParser::new(&map);
    for el in parser.verb_to_base(&verbs){
        println!("{}", vocab_vector[el as usize]);
    }
    // let Some(n) = return_some_option() else { return };
}
