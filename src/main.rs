const VOWELS: [char; 7] = ['a', 'e', 'ë', 'i', 'o', 'u', 'y'];

fn suffix_to_possibilities(suffix: &str) -> Option<Vec<&str>> {
    // "" means the suffix is removed,
    // None means try sth else
    let mat = match suffix {
        // Even different arms that return the same thing, the Rust optmizer treats them as the same thing
        // This way we can keep different logic with the same result in different arms for clarity
        //zgj1. kr.thj + pjesore
        "ova" | "ove" | "oi" | "uam" | "uat" | "uan" | "uar" => vec!["oj", "uaj"],
        "va" | "ve" | "u" | "më" | "të" | "në" => vec!["j", "e", ""], // + "e" per shtie? but really low
                                                             // probability on that ngl.
                                                             // The empty option per "pi"
                                                             // Let's be as inclusive as possible
                                                             // and discuss probabilities later
        "rë" => vec!["j"], // larë and bërë -> are different in some forms (lava, bëra) but the same in this
        "ja" | "je" | "nim" | "nit" | "nin" => vec!["j", "", "e"], // + "e" per shtie?
        "nte" => vec!["j"],
        "j" | "n" | "jmë" | "jne" => vec!["j"],
        "ni" => vec!["j"], // shkruani, vendosni => different in some forms but the same here
        "ta" | "ti" | "tëm" | "tët" | "tën" | "tur" => vec!["j", ""], // "" per di -> dita
                                                                      // and friends
        "te" => vec!["j", ""], // arrite, vendoste => different groupings
        "im" | "in" => vec![""],
        "ra" | "ri" => vec!["j"],
        _ => return None, // Now that I think about it, if it matches none of these, then it just
                          // might be simply one that is okay the way it is ""
                          // We will need to match in the very beginning if it is the base form
                          // though. So this is fine
    };
    Some(mat)
}

fn main() {
    println!("Hello, world!");
    verb_to_base("punojme");
    let Some(_) = suffix_to_possibilities("morea") else { return };
    println!("This is never reached");
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
