use fst::{
    IntoStreamer, Map, Streamer,
    raw::{Fst, Output},
};

use crate::alb_parser_imprv::SearchState;

pub fn get_id_fuzzy(map: &Map<&[u8]>, input: &[u8]) -> Option<u16> {
    // 1. Clean the input (trim non-alphanumeric from sides only)
    let input = unsafe { std::str::from_utf8_unchecked(input) };
    let clean_input = input.trim_matches(|c: char| !c.is_alphanumeric());

    // 2. Try exact match first (Most efficient)
    if let Some(id) = map.get(clean_input) {
        return Some(id as u16);
    }

    // // 3. Fallback: Levenshtein for typos/extra spaces
    // // Distance 1 handles one extra space or one wrong character
    // let lev = fst::automaton::Levenshtein::new(clean_input, 1).ok()?;
    // let mut stream = map.search(lev).into_stream();

    // // Grab the first alphabetical match found by the automaton
    // if let Some((_key, id)) = stream.next() {
    //     return Some(id as u16);
    // }

    None
}

pub fn get_category_id(fst: &Fst<&[u8]>, verb: &[u8], stack: &mut Vec<SearchState>) -> Option<u16> {
    // We map the tuple (ID, length) to just the ID, casting to u16 as requested.
    let val = find_longest_match_with_rules(fst, verb, stack);
    val.map(|(id, _len)| id as u16)
}

fn find_longest_match_with_rules(
    fst: &Fst<&[u8]>,
    input: &[u8],
    stack: &mut Vec<SearchState>,
) -> Option<(usize, usize)> {
    let mut last_found: Option<(usize, usize)> = None;

    stack.clear();
    // Initialize stack with the root node
    stack.push(SearchState {
        node_addr: fst.root().addr(),
        input_idx: 0,
        output: Output::zero(),
    });

    // We loop until there are no paths left to explore
    while let Some(state) = stack.pop() {
        // Reconstruct the node from the raw address
        let node = fst.node(state.node_addr);

        // 1. Check if the current node is a Final State (Match Found)
        if node.is_final() {
            let final_val = state.output.cat(node.final_output()).value() as usize;

            // Logic to determine if this match is "better"
            match last_found {
                None => last_found = Some((final_val, state.input_idx)),
                Some((_, best_len)) => {
                    // We typically prioritize the longest match input-wise
                    if state.input_idx > best_len {
                        last_found = Some((final_val, state.input_idx));
                    }
                }
            }
        }
        // Stop if we have exhausted the input for this path
        if state.input_idx >= input.len() {
            continue;
        }

        let first_byte = input[state.input_idx];

        let mut current_byte = first_byte.to_ascii_lowercase();

        let mut bytes_to_consume = 1;

        // --- UTF-8 Multi-byte Handling ---
        // 0xC3 is the prefix for many accented Latin characters
        if first_byte == 0xC3 && state.input_idx + 1 < input.len() {
            let second_byte = input[state.input_idx + 1];

            match second_byte {
                // 0x87 (Ç) or 0xA7 (ç) -> normalize to 'c'
                0xA7 => {
                    current_byte = b'c';
                    bytes_to_consume = 2;
                }
                // 0x8B (Ë) or 0xAB (ë) -> normalize to 'e'
                0xAB => {
                    current_byte = b'e';
                    bytes_to_consume = 2;
                }
                // Add other 0xC3 cases here if needed (e.g., 0xA9 for 'é')
                _ => {}
            }
        }

        if state.input_idx + 1 < input.len() && current_byte == b' ' {
            stack.push(SearchState {
                node_addr: state.node_addr,
                input_idx: state.input_idx + 1,
                output: state.output,
            });
        }

        // --- STANDARD PATH: Exact Match ---
        // We push this last so it pops first.
        if let Some(idx) = node.find_input(current_byte) {
            let trans = node.transition(idx);
            stack.push(SearchState {
                node_addr: trans.addr,
                input_idx: state.input_idx + bytes_to_consume,
                output: state.output.cat(trans.out),
            });
        }
    }

    last_found
}

// #[inline]
// fn trim_u8_alphanumeric(bytes: &[u8]) -> &[u8] {
//     let start = bytes.iter().position(|b| b.is_ascii_alphanumeric()).unwrap_or(0);
//     let end = bytes.iter().rposition(|b| b.is_ascii_alphanumeric()).unwrap_or(0);

//     if start > end {
//         &[]
//     } else {
//         &bytes[start..=end]
//     }
// }

