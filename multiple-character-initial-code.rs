//multiple character initial code

// fn suffix_to_possibilities(suffix: &str) -> Option<Vec<&str>> {
//     // "" means the suffix is removed,
//     // None means try sth else
//     let mat = match suffix {
//         // Even different arms that return the same thing, the Rust optmizer treats them as the same thing
//         // This way we can keep different logic with the same result in different arms for clarity
//         //zgj1. kr.thj + pjesore
//         "ova" | "ove" | "oi" | "uam" | "uat" | "uan" | "uar" => vec!["oj", "uaj"],
//         "jta" | "jte" | "jti" | "jtëm" | "jtët" | "jtën" | "jtur" => vec!["j"], // mbajta and
//         // friends, easy
//         // match but waste
//         // of performance,
//         // maybe?
//         "va" | "ve" | "u" | "më" | "të" | "në" => vec!["j", "e", ""], // + "e" per shtie? but really low
//         // probability on that ngl.
//         // The empty option per "pi"
//         // Let's be as inclusive as possible
//         // and discuss probabilities later
//         "rë" => vec!["j"], // larë and bërë -> are different in some forms (lava, bëra) but the same in this
//         "ja" | "je" | "nim" | "nit" | "nin" => vec!["j", "", "e"], // + "e" per shtie?
//         // edhe "" por ndryshon zanorja
//         // e fundit nga i ne ë (vija ->
//         // vë)
//         "nte" => vec!["j"],
//         "j" | "n" | "jmë" | "jne" => vec!["j"],
//         "ni" => vec!["j"], // shkruani, vendosni => different in some forms but the same here
//         "ta" | "ti" | "tëm" | "tët" | "tën" | "tur" => vec!["j", ""], // "" per di -> dita
//         // and friends
//         "te" => vec!["j", ""], // arrite, vendoste => different groupings
//         "im" | "in" => vec![""],
//         "ra" | "ri" => vec!["j"],
//         "a" | "e" | "i" | "ëm" | "ët" | "ën" | "ë" | "ur" => vec![""], // This needs special
//         // treatment however,
//         // because of words like
//         // dal dhe heq that change
//         // their vowel in past
//         // simple
//         _ => return None, // Now that I think about it, if it matches none of these, then it just
//                           // might be simply one that is okay the way it is ""
//                           // We will need to match in the very beginning if it is the base form
//                           // though. So this is fine
//     };
//     Some(mat)
// }
