use std::vec;

use unicode_normalization::UnicodeNormalization;

// const VOWELS: [char; 7] = ['a', 'e', 'ë', 'i', 'o', 'u', 'y'];
// // the letter ë has two ways of being expressed that we should account for:
// const NFC_E_DIAERESIS_2B: [u8; 2] = [0xC3, 0xAB];
// const NFD_E_DIAERESIS_3B: [u8; 3] = [0x65, 0xCC, 0x88];
// When converted to &str: "ë"

// fn suffix_1char(ch: char) -> Option<Vec<&'static str>> {
//     // For now, I'm keeping it simple with
//     // static lifetimes
//     let mat = match ch {
//         'u' => vec!["j", "e", ""],
//         'j' | 'n' => vec!["j"],
//         'a' | 'e' | 'i' | 'ë' => vec![""],
//         _ => return None,
//     };
//     Some(mat)
// }
fn suffix_1char(ch: &str) -> Option<Vec<&'static str>> {
    // For now, I'm keeping it simple with
    // static lifetimes
    let mat = match ch {
        "u" => vec!["j", "e", ""],
        "j" | "n" => vec!["j"],
        "a" | "e" | "i" | "ë" => vec![""],
        _ => return None,
    };
    Some(mat)
}
fn suffix_2char(st: &str) -> Option<Vec<&'static str>> {
    // only the last two elements of the string are passed
    let mat = match st {
        "oi" => vec!["oj", "uaj"],
        "va" | "ve" | "më" | "të" | "në" => vec!["j", "e", ""], // + "e" per shtie? but really low
        "rë" => vec!["j"],
        "ja" | "je" => vec!["j", "", "e"],
        "ni" => vec!["j"],
        "ta" | "ti" => vec!["j", ""],
        "te" => vec!["j", ""],
        "im" | "in" => vec![""],
        "ra" | "ri" => vec!["j"],
        "ëm" | "ët" | "ën" | "ur" => vec![""], // kto me ë psh do kalohen te ato qe duan 3
        // karaktere
        _ => return None,
    };
    Some(mat)
}
fn suffix_3char(st: &str) -> Option<Vec<&'static str>> {
    let mat = match st {
        "ova" | "ove" | "uam" | "uat" | "uan" | "uar" => vec!["oj", "uaj"],
        "jta" | "jte" | "jti" => vec!["j"],
        "nim" | "nit" | "nin" => vec!["j", "", "e"],
        "nte" => vec!["j"],
        "jmë" | "jne" => vec!["j"],
        "tëm" | "tët" | "tën" => vec!["j", ""],
        "tur" => vec!["j", ""],
        _ => return None,
    };
    Some(mat)
}

fn suffix_4char(st: &str) -> Option<Vec<&'static str>> {
    let mat = match st {
        "jtëm" | "jtët" | "jtën" | "jtur" => vec!["j"],
        _ => return None,
    };
    Some(mat)
}

fn main() {
    println!("Hello, world!");
    let verbs = vec!["punojmë", "punuam", "shkruar", "lexuar", "vendosur"];
    verb_to_base(&verbs);
    // let Some(n) = return_some_option() else { return };
}

fn verb_to_base(verbs: &[&str]) { // A slice of slices (kind of like an array of arrays)
    let mut normalization_buffer = String::with_capacity(256);
    // Called main buffer because it's what's left after normalization and lowercasing
    let mut main_buffer = String::with_capacity(256); 
    let mut base_form = String::with_capacity(256);
    for &verb in verbs {
        // making sure the buffers are empty
        normalization_buffer.clear();
        main_buffer.clear();
        base_form.clear();
        // adding the characters
        // the function below normalizes three byte ë and ç to the 2 byte version.
        normalization_buffer.extend(verb.nfc());
        // I don't like it but I was basically forced to use two different buffers.
        main_buffer.extend(
            // Had to do this because lowercasing would create a String otherwise.
            // I still have to read the assembly. If the assembly is bad, I'll implement it myself
            normalization_buffer.chars().flat_map(|ch| ch.to_lowercase()),
        );
        base_form.push_str(&main_buffer); // This is the variable we will test
        // having the suffix functions be Option is looking like it was a terrible idea as of now
        // I'm keeping this simple for now, usually the logic to get the next slice would be a
        // little bit more advanced. This way I don't have to check the lengths manually though
        // starting with 4 characters
        possibilities_for_verb(4, &main_buffer, &mut base_form, suffix_4char);
        base_form.clear();
        base_form.push_str(&main_buffer); // This part will also need to be improved.
        possibilities_for_verb(3, &main_buffer, &mut base_form, suffix_3char);
        base_form.clear();
        base_form.push_str(&main_buffer);
        possibilities_for_verb(2, &main_buffer, &mut base_form, suffix_2char);
        base_form.clear();
        base_form.push_str(&main_buffer);
        possibilities_for_verb(1, &main_buffer, &mut base_form, suffix_1char);
        // 2 characters
    }
}

type SuffixFunction = fn(&str) -> Option<Vec<&str>>;

fn possibilities_for_verb(
    suffix_char_len: usize,
    main_buffer: &str, // This at the very least needs a better name
    base_form: &mut String,
    suffix_function: SuffixFunction,
) {
    // usually you know how much I don't like unnecessary functions but this is repeated 4 times,
    // and this way it probably has more instruction cache advantages
    if let Some((suffix_offset, _ch)) = main_buffer.char_indices().nth_back(suffix_char_len - 1) {
        let suffix = &main_buffer[suffix_offset..];
        println!("Verb: {}, Suffix: {} => ", main_buffer, suffix);
        if let Some(possibilities) = suffix_function(suffix) {
            for el in possibilities {
                base_form.replace_range(suffix_offset.., el);
                println!("{}", base_form);
            }
        }
    }
}
// fn return_some_option() -> Option<i32> {
//     Option::Some(234903)
// }
