use std::vec;

use unicode_normalization::UnicodeNormalization;

const VOWELS: [char; 7] = ['a', 'e', 'ë', 'i', 'o', 'u', 'y'];
// the letter ë has two ways of being expressed that we should account for:
const NFC_E_DIAERESIS_2B: [u8; 2] = [0xC3, 0xAB];
const NFD_E_DIAERESIS_3B: [u8; 3] = [0x65, 0xCC, 0x88];
// When converted to &str: "ë"

fn suffix_to_possibilities(suffix: &str) -> Option<Vec<&str>> {
    // "" means the suffix is removed,
    // None means try sth else
    let mat = match suffix {
        // Even different arms that return the same thing, the Rust optmizer treats them as the same thing
        // This way we can keep different logic with the same result in different arms for clarity
        //zgj1. kr.thj + pjesore
        "ova" | "ove" | "oi" | "uam" | "uat" | "uan" | "uar" => vec!["oj", "uaj"],
        "jta" | "jte" | "jti" | "jtëm" | "jtët" | "jtën" | "jtur" => vec!["j"], // mbajta and
        // friends, easy
        // match but waste
        // of performance,
        // maybe?
        "va" | "ve" | "u" | "më" | "të" | "në" => vec!["j", "e", ""], // + "e" per shtie? but really low
        // probability on that ngl.
        // The empty option per "pi"
        // Let's be as inclusive as possible
        // and discuss probabilities later
        "rë" => vec!["j"], // larë and bërë -> are different in some forms (lava, bëra) but the same in this
        "ja" | "je" | "nim" | "nit" | "nin" => vec!["j", "", "e"], // + "e" per shtie?
        // edhe "" por ndryshon zanorja
        // e fundit nga i ne ë (vija ->
        // vë)
        "nte" => vec!["j"],
        "j" | "n" | "jmë" | "jne" => vec!["j"],
        "ni" => vec!["j"], // shkruani, vendosni => different in some forms but the same here
        "ta" | "ti" | "tëm" | "tët" | "tën" | "tur" => vec!["j", ""], // "" per di -> dita
        // and friends
        "te" => vec!["j", ""], // arrite, vendoste => different groupings
        "im" | "in" => vec![""],
        "ra" | "ri" => vec!["j"],
        "a" | "e" | "i" | "ëm" | "ët" | "ën" | "ë" | "ur" => vec![""], // This needs special
        // treatment however,
        // because of words like
        // dal dhe heq that change
        // their vowel in past
        // simple
        _ => return None, // Now that I think about it, if it matches none of these, then it just
                          // might be simply one that is okay the way it is ""
                          // We will need to match in the very beginning if it is the base form
                          // though. So this is fine
    };
    Some(mat)
}

fn suffix_1char(ch: char) -> Option<Vec<&'static str>> {
    // For now, I'm keeping it simple with
    // static lifetimes
    let mat = match ch {
        'u' => vec!["j", "e", ""],
        'j' | 'n' => vec!["j"],
        'a' | 'e' | 'i' | 'ë' => vec![""],
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
    let verbs = vec!["punojmë", "punoni"];
    verb_to_base(&verbs);
    let Some(_) = suffix_to_possibilities("morea") else { return };
    println!("This is never reached");
    // let Some(n) = return_some_option() else { return };
}

fn verb_to_base(verbs: &[&str]) {
    // A slice of slices (kind of like an array of arrays)
    let mut normalization_buffer = String::with_capacity(256);
    let mut to_lowercase_buffer = String::with_capacity(256);
    for &verb in verbs {
        // making sure the buffers are empty
        normalization_buffer.clear();
        to_lowercase_buffer.clear();
        // adding the characters
        normalization_buffer.extend(verb.nfc());
        // I don't like it but I feel like I was forced to use two different buffers.
        to_lowercase_buffer.extend( // Had to do this because lowercasindsag created a String
                                    // otherwise.
                                    // I still have to read the assembly
                                    // If the assembly is bad, I'll implement it myself
            normalization_buffer
                .chars()
                .flat_map(|ch| ch.to_lowercase()),
        ); 
        // 4 characters
        // 3 characters
        // 2 characters
    }
}

// fn return_some_option() -> Option<i32> {
//     Option::Some(234903)
// }
