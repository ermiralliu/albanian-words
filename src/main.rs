const VOWELS: [char; 7] = ['a', 'e', 'ë', 'i', 'o', 'u', 'y'];
// Ë
// zgj1
// Part 1 -> verbs that take a 'v' between the root and suffix
// 1.1
//
// 1.2 --> That in simple past:
// 'ova' => 'uaj'
// 'eva' => 'yej'

fn suffix_to_possibilities(suffix: &str) -> Vec<&str> {
    // I'm doing this so I can see what matches
    // are repeated.
    match suffix {
        "ova" | "ove" | "oi" | "uam" | "uat" | "uan" | "uar" => vec!["oj", "uaj"],
        "va" | "ve" | "u" | "më" | "të" | "në" | "rë" => vec!["j"],
        "ja" | "je" | "nte" | "nim" | "nit" | "nin" => vec!["j"],
        "j" | "n" | "jmë" | "ni" | "jne" => vec!["j"],
        "ta" | "te" | "ti" | "tëm" | "tët" | "tën" | "tur" => vec!["j"],
        "ra" | "rë" | "ri" => vec!["j"], // Leaving problems on for now, to sort them out later
        _ => vec![], // I'll return a Option(None) instead
    }
}

fn main() {
    println!("Hello, world!");
    verb_to_base("punojme");
    // let Some(n) = return_some_option() else { return };
}

fn verb_to_base(verb: &str) {
    // 4 characters
    // 3 characters
    // 2 characters
    let two = verb;
    println!("{two}")
}

// fn return_some_option() -> Option<i32> {
//     Option::Some(234903)
// }
