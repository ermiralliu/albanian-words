use std::collections::HashMap;
use unicode_normalization::UnicodeNormalization;

type SuffixFunction = fn(&str) -> Option<&[&str]>;

// using &[u8] lets you have fun without worrying about sizes. Nice Rust stuff.
// em dash might also appear, but I think that shit will likely have spaces around, so who cares.
const WORD_DELIMITERS: &[u8] = &[
    b'.', b',', b'/', b'\\', b' ', b'\n', b'\t', b'\"', b'\'', b':', b';', b'!', b'?', b'(', b')', b'[', b']', 
    b'\r',
];

const CONTENT_DELIMITER: &str = "/endarticle";

const CATEGORY_DELIMITER: u8 = b'\n';

const DEFAULT_WORD_BUFFER_CAPACITY: usize = 256;

pub struct AlbanianParser<'a> {
    vocab: &'a HashMap<&'a str, u16>,
    normalization_buffer: String, // after normalizing ë andç
    // main_buffer: String,          // after lowercasing the normalization buffer
    base_form: String,            // after lowercasing the normalization buffer
}

impl<'a> AlbanianParser<'a> {
    pub fn new(vocab: &'a HashMap<&'a str, u16>) -> AlbanianParser<'a> {
        AlbanianParser {
            vocab,
            normalization_buffer: String::with_capacity(DEFAULT_WORD_BUFFER_CAPACITY),
            // main_buffer: String::with_capacity(DEFAULT_WORD_BUFFER_CAPACITY),
            base_form: String::with_capacity(DEFAULT_WORD_BUFFER_CAPACITY),
        }
    }
    /// Returns a vector of ids for a sentence
    // pub fn verb_to_base(&mut self, verbs: &[&str]) -> Vec<u16> {
    //     let mut ids = Vec::new(); // not providing u16 directly to try keep it more open
    //     for &verb in verbs {
    //         // making sure the buffers are empty
    //         self.normalization_buffer.clear();
    //         self.main_buffer.clear();
    //         self.base_form.clear();
    //         // adding the characters
    //         // the function below normalizes three byte ë and ç to the 2 byte version.
    //         self.normalization_buffer.extend(verb.nfc());
    //         // I don't like it but I was basically forced to use two different buffers.
    //         self.main_buffer.extend(
    //             // Had to do this because lowercasing would create a String otherwise.
    //             // I still have to read the assembly. If the assembly is bad, I'll implement it myself
    //             self.normalization_buffer.chars().flat_map(|ch| ch.to_lowercase()),
    //         );
    //         self.base_form.push_str(&self.main_buffer); // This is the variable we will test
    //         if let Some(id) = self.possibilities_for_verb(4, suffix_4char) {
    //             ids.push(id);
    //             continue;
    //         }
    //         self.base_form.replace_range(.., &self.main_buffer);
    //         // self.base_form.clear();
    //         // self.base_form.push_str(&self.main_buffer); // This part will also need to be improved.
    //         if let Some(id) = self.possibilities_for_verb(3, suffix_3char) {
    //             ids.push(id);
    //             continue;
    //         }
    //         self.base_form.replace_range(.., &self.main_buffer);
    //         if let Some(id) = self.possibilities_for_verb(2, suffix_2char) {
    //             ids.push(id);
    //             continue;
    //         }
    //         self.base_form.replace_range(.., &self.main_buffer);
    //
    //         if let Some(id) = self.possibilities_for_verb(1, suffix_1char) {
    //             ids.push(id);
    //         }
    //     }
    //     ids
    // }

    pub fn single_verb_to_base(&mut self, verb: &str) -> Option<u16> {
        // self.normalization_buffer.clear();
        // self.main_buffer.clear();
        // self.base_form.clear();

        // self.normalization_buffer.extend(verb.nfc());
        // // I don't like it but I was basically forced to use two different buffers.
        // self.main_buffer.extend(
        //     // Had to do this because lowercasing would create a String otherwise.
        //     // I still have to read the assembly. If the assembly is bad, I'll implement it myself
        //     self.normalization_buffer.chars().flat_map(|ch| ch.to_lowercase()),
        // );
        // self.main_buffer.push_str(verb); // this probably doesn't need to be pushed
                                                 // and should be used directly
        // Yeah, I'm passing verb directly into the function now.
        self.base_form.clear();
        self.base_form.push_str(verb); // This is the variable we will test
        const CHECKS: &[(usize, SuffixFunction)] = &[
            (4, suffix_4char),
            (3, suffix_3char),
            (2, suffix_2char),
            (1, suffix_1char),
        ];

        CHECKS
            .iter()
            .find_map(|&(len, func)| self.possibilities_for_verb(verb, len, func))
    }

    fn possibilities_for_verb(&mut self, verb: &str, suffix_char_len: usize, suffix_function: SuffixFunction) -> Option<u16> {
        // usually you know how much I don't like unnecessary functions but this is repeated 4 times,
        // and this way it probably has more instruction cache advantages
        if verb.len() <= suffix_char_len { return None; }; // Somehow I didn't have a single guard
                                                           // here?
        let main_buffer = verb;
        if let Some((suffix_offset, _ch)) = main_buffer.char_indices().nth_back(suffix_char_len - 1) {
            let suffix = &main_buffer[suffix_offset..];
            #[cfg(debug_assertions)]
            {
                // println!("Verb: {}, Suffix: {}", main_buffer, suffix);
            }
            if let Some(possibilities) = suffix_function(suffix) {
                for el in possibilities {
                    self.base_form.replace_range(suffix_offset.., el);
                    // println!("{}",self.base_form);
                    if let Some(matching_word) = self.vocab.get(&self.base_form[..]) {
                        // simple way of verifying a value and removing it in
                        // production code.
                        #[cfg(debug_assertions)]
                        {
                            // let base_form = &self.base_form;
                            // dbg!(base_form);
                        }
                        self.base_form.replace_range(.., &main_buffer);
                        return Some(*matching_word); // we return on the first match. This is a bit
                        // this is a bit tightly coupled ngl
                    };
                }
                self.base_form.replace_range(.., &main_buffer);
                // I wish there was a way that doesn't require setting this twice.
            }
        }
        None
    }
}

// type SuffixFunction = fn(&str) -> Option<Vec<&str>>;

fn suffix_1char(ch: &str) -> Option<&[&'static str]> {
    // For now, I'm keeping it simple with
    // static lifetimes
    let mat: &[&str] = match ch {
        "u" => &["j", "e", ""],
        "j" | "n" => &["j"],
        "a" | "e" | "i" | "ë" => &[""],
        _ => return None,
    };
    Some(mat)
}
fn suffix_2char(st: &str) -> Option<&[&'static str]> {
    // only the last two elements of the string are passed
    let mat: &[&str] = match st {
        // this one needed explicit coercion
        "oi" => &["oj", "uaj"],
        "va" | "ve" | "më" | "të" | "në" => &["j", "e", ""], // + "e" per shtie? but really low
        "rë" => &["j"],
        "ja" | "je" => &["j", "", "e"],
        "ni" => &["j"],
        "ta" | "ti" => &["j", ""],
        "te" => &["j", ""],
        "im" | "in" => &[""],
        "ra" | "ri" => &["j"],
        "ëm" | "ët" | "ën" | "ur" => &[""], // kto me ë psh do kalohen te ato qe duan 3
        // karaktere
        _ => return None,
    };
    Some(mat)
}
fn suffix_3char(st: &str) -> Option<&[&'static str]> {
    let mat: &[&str] = match st {
        "ova" | "ove" | "uam" | "uat" | "uan" | "uar" => &["oj", "uaj"],
        "jta" | "jte" | "jti" => &["j"],
        "nim" | "nit" | "nin" => &["j", "", "e"],
        "nte" => &["j"],
        "jmë" | "jne" => &["j"],
        "tëm" | "tët" | "tën" => &["j", ""],
        "tur" => &["j", ""],
        _ => return None,
    };
    Some(mat)
}

fn suffix_4char(st: &str) -> Option<&[&'static str]> {
    let mat: &[&str] = match st {
        "jtëm" | "jtët" | "jtën" | "jtur" => &["j"],
        _ => return None,
    };
    Some(mat)
}
