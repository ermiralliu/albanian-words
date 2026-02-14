use fst::raw::{Fst, Output};

pub fn single_verb_to_base_new(fst: &Fst<&[u8]>, verb: &[u8]) -> Option<u16> {
    let val = find_longest_match(fst, verb);
    val.map(|(id, _len)| id as u16)
}

fn find_longest_match(fst: &Fst<&[u8]>, input: &[u8]) -> Option<(usize, usize)> {
    let mut node = fst.root();
    let mut out = Output::zero();
    let mut last_found = None;

    for (i, &byte) in input.iter().enumerate() {
        // Find the specific transition for this byte
        if let Some(index) = node.find_input(byte) {
            let transition = node.transition(index);
            // Accumulate the value along the path
            out = out.cat(transition.out);
            // Move to the next state (node)
            node = fst.node(transition.addr);

            if node.is_final() {
                // Total value = path values + the final state's weight
                let final_id = out.cat(node.final_output()).value() as usize;
                last_found = Some((final_id, i + 1));
            }
        } else {
            break;
        }
    }
    last_found
}

pub fn single_verb_to_base_ultra(fst: &Fst<&[u8]>, verb: &[u8], stack: &mut Vec<SearchState>) -> Option<u16> {
    // We map the tuple (ID, length) to just the ID, casting to u16 as requested.
    let val = find_longest_match_with_rules(fst, verb, stack);
    val.map(|(id, _len)| id as u16)
}

/// A state representing a position in the search logic.
#[derive(Clone, Copy)]
pub struct SearchState {
    node_addr: usize, // The raw address of the FST node
    input_idx: usize, // Current index in the input string
    output: Output,   // Accumulated value so far
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

        let current_byte = input[state.input_idx];

        // --- RULE IMPLEMENTATION STRATEGY ---
        // We push states to the stack. Because it's LIFO (Last In, First Out),
        // we push the "Fuzzy" rules FIRST (so they are processed last/lazily)
        // and the "Exact" match LAST (so it is processed immediately).

        // --- RULE 1: Skip `ë` between consonants ---
        // Assuming UTF-8, 'ë' is [0xC3, 0xAB]. We check for this sequence.
        if state.input_idx + 1 < input.len() && current_byte == 0xC3 && input[state.input_idx + 1] == 0xAB {
            // Check context: Prev char exists & is consonant, Next char (after ë) exists & is consonant
            // Note: `input_idx` is at 0xC3. `input_idx + 2` is the char after ë.
            if state.input_idx > 0
                && state.input_idx + 2 < input.len()
                && is_consonant(input[state.input_idx - 1])
                && is_consonant(input[state.input_idx + 2])
            {
                // Push a state that advances input by 2 (skips 'ë')
                // BUT stays on the SAME FST node.
                stack.push(SearchState {
                    node_addr: state.node_addr,
                    input_idx: state.input_idx + 2,
                    output: state.output,
                });
            }
        }

        // --- RULE 2: Swap `u` -> `o` ---
        // If input is 'u', we allow the FST to traverse 'o'
        if current_byte == b'u' {
            // Check if the current FST node actually has a transition for 'o'
            if let Some(idx) = node.find_input(b'o') {
                let trans = node.transition(idx);
                stack.push(SearchState {
                    node_addr: trans.addr,
                    input_idx: state.input_idx + 1, // Consume 'u' from input
                    output: state.output.cat(trans.out),
                });
            }
        }

        // --- STANDARD PATH: Exact Match ---
        // We push this last so it pops first.
        if let Some(idx) = node.find_input(current_byte) {
            let trans = node.transition(idx);
            stack.push(SearchState {
                node_addr: trans.addr,
                input_idx: state.input_idx + 1,
                output: state.output.cat(trans.out),
            });
        }
    }

    last_found
}

/// Helper to check simple consonants (ASCII approximation)
fn is_consonant(b: u8) -> bool {
    matches!(
        b,
        b'b' | b'c'
            | b'd'
            | b'f'
            | b'g'
            | b'h'
            | b'j'
            | b'k'
            | b'l'
            | b'm'
            | b'n'
            | b'p'
            | b'q'
            | b'r'
            | b's'
            | b't'
            | b'v'
            | b'x'
            | b'z' // Add uppercase if needed, or specific Albanian chars if they are single byte
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use fst::MapBuilder;
    use fst::raw::Fst;

    /// Helper to create a temporary FST for testing
    fn setup_test_fst() -> Vec<u8> {
        let mut builder = MapBuilder::memory();
        // Keys MUST be inserted in lexicographical order
        builder.insert("hap", 10).unwrap();
        builder.insert("libr", 20).unwrap();
        builder.insert("puno", 30).unwrap();
        builder.into_inner().unwrap()
    }

    #[test]
    fn test_exact_match() {
        let data = setup_test_fst();
        let fst = Fst::new(&data[..]).unwrap();
        let mut stack = Vec::new();
        // "hap" should match exactly
        assert_eq!(single_verb_to_base_ultra(&fst, b"hap", &mut stack), Some(10));
    }

    #[test]
    fn test_skip_e_rule() {
        let data = setup_test_fst();
        let fst = Fst::new(&data[..]).unwrap();

        let mut stack = Vec::new();
        // "libër" contains ë (0xC3 0xAB) between 'b' and 'r'
        // The FST only contains "libr", so the rule should skip 'ë'
        assert_eq!(single_verb_to_base_ultra(&fst, "libër".as_bytes(), &mut stack), Some(20));
    }

    #[test]
    fn test_u_to_o_swap() {
        let data = setup_test_fst();
        let fst = Fst::new(&data[..]).unwrap();
        let mut stack = Vec::new();
        // Input "punu" should match FST entry "puno"
        assert_eq!(single_verb_to_base_ultra(&fst, b"punu", &mut stack), Some(30));

        // Prefix test: "punuar" should match the longest valid FST path "puno"
        assert_eq!(single_verb_to_base_ultra(&fst, b"punuar", &mut stack), Some(30));
    }

    #[test]
    fn test_no_match() {
        let data = setup_test_fst();
        let fst = Fst::new(&data[..]).unwrap();
        let mut stack = Vec::new();

        // Random word that shouldn't match
        assert_eq!(single_verb_to_base_ultra(&fst, b"shko", &mut stack), None);
    }
}
