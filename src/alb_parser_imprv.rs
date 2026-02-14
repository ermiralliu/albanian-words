use fst::{
    raw::{Fst, Output},
};

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
        // }
        // else if is_vowel(byte) {
        //     // Vowel dropping: Skip the input byte, stay in the current node
        //     continue;
        } else {
            break;
        }
    }
    last_found
}
