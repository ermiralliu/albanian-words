use std::collections::HashMap;
// use unicode_normalization::UnicodeNormalization;

// type SuffixFunction = fn(&str) -> Option<&[&str]>;
type SuffixFunction = fn(u64) -> Option<&'static [&'static [u8]]>;

// using &[u8] lets you have fun without worrying about sizes. Nice Rust stuff.

const DEFAULT_WORD_BUFFER_CAPACITY: usize = 32; // Increased this size only because of some
// retarded articles

pub struct AlbanianParser<'a> {
    vocab: &'a HashMap<&'a [u8], u16>,
    // normalization_buffer: String, // after normalizing ë and ç
    // main_buffer: String,          // after lowercasing the normalization buffer
    base_form: [u8; DEFAULT_WORD_BUFFER_CAPACITY], // after lowercasing the normalization buffer
    base_form_len: usize,
}

impl<'a> AlbanianParser<'a> {
    pub fn new(vocab: &'a HashMap<&'a [u8], u16>) -> AlbanianParser<'a> {
        AlbanianParser {
            vocab,
            base_form: [0u8; DEFAULT_WORD_BUFFER_CAPACITY],
            base_form_len: 0,
        }
    }
    #[inline]
    pub fn single_verb_to_base(&mut self, verb: &[u8]) -> Option<u16> {
        // we can use copy non-overlapping if the copy below is not enough
        if verb.len() >= 32 { // I was wondering how to deal with it but yeah. Just return nothing.
                              // The largest albanian word is less
            return None;
        }
        self.base_form[..verb.len()].copy_from_slice(verb);
        let len = verb.len();
        self.base_form_len = len;
        const CHECKS: &[(usize, SuffixFunction)] = &[
            (5, suffix_5byte),
            (4, suffix_4byte),
            (3, suffix_3byte),
            (2, suffix_2byte),
            (1, suffix_1byte),
        ];
        let initial_suffix = match len {
            0..=2 => return None,
            3..=6 => byte_arr_to_nr(&verb[2..]),
            7.. => byte_arr_to_nr(&verb[len - 5..]),
        };

        CHECKS
            .iter()
            .filter(|&(length, _)| length < &verb.len())
            .find_map(|&(len, func)| self.possibilities_for_verb(verb, initial_suffix, len, func))
    }
    #[inline]
    fn possibilities_for_verb(
        // this part is a little bit too much for what it's doing
        &mut self,
        verb: &[u8],
        initial_suffix: u64,
        suffix_byte_len: usize,
        suffix_function: SuffixFunction,
    ) -> Option<u16> {
        // usually you know how much I don't like unnecessary functions but this is repeated 4 times,
        // and this way it probably has more instruction cache advantages
        let main_buffer = verb;
        if main_buffer.len() >= suffix_byte_len {
            let suffix_offset = main_buffer.len() - suffix_byte_len;
            // let suffix = &main_buffer[suffix_offset..];

            if let Some(possibilities) = suffix_function(initial_suffix) {
                for el in possibilities {
                    let el_bytes = el;

                    // 1. "Replace" by writing the new suffix bytes over the old ones
                    unsafe {
                        std::ptr::copy_nonoverlapping(
                            el_bytes.as_ptr(),
                            self.base_form.as_mut_ptr().add(suffix_offset),
                            el_bytes.len(),
                        );
                    }

                    // 2. Update the length to account for the new suffix length
                    let current_total_len = suffix_offset + el_bytes.len();

                    // 3. Lookup in vocab using a slice of the array
                    if let Some(matching_word) = self.vocab.get(&self.base_form[..current_total_len]) {
                        return Some(*matching_word);
                    }

                    // Note: We don't need to "reset" the buffer inside the loop because
                    // the next 'el' will simply overwrite the same suffix_offset.
                }
            }
        }
        None
    }
}

// I initially forgot that these have to be in little endian
const LAST_4_BYTES: u64 = 0xFFFF_FFFF_0000_0000;
const LAST_3_BYTES: u64 = 0xFFFF_FF_0000_0000;
const LAST_2_BYTES: u64 = 0xFFFF_0000_0000_0000;
const LAST_BYTE: u64 = 0xFF00_0000_0000_0000;

const fn byte_arr_to_nr(st: &[u8]) -> u64 {
    let mut s = 0u64; // Initialize 8 bytes of zeros
    let len = st.len();
    let dest_start = unsafe { (&mut s as *mut u64 as *mut u8).add(8 - len) };

    unsafe {
        // This is a direct raw pointer copy (memcpy)
        std::ptr::copy_nonoverlapping(st.as_ptr(), dest_start, len);
    }
    s
}

const U: u64 = byte_arr_to_nr(b"u");
const J: u64 = byte_arr_to_nr(b"j");
const N: u64 = byte_arr_to_nr(b"n");
const A: u64 = byte_arr_to_nr(b"a");
const E: u64 = byte_arr_to_nr(b"e");
const I: u64 = byte_arr_to_nr(b"i");

fn suffix_1byte(ch: u64) -> Option<&'static [&'static [u8]]> {
    // For now, I'm keeping it simple with
    // static lifetimes
    let final_byte = ch & LAST_BYTE;
    let mat: &[&[u8]] = match final_byte {
        U => &[b"j", b"e", b""],
        J | N => &[b"j"],
        A | E | I => &[b""],
        _ => return None,
    };
    Some(mat)
}

const E_DIAERESIS: u64 = u64::from_le_bytes([0xC3, 0xAB, 0, 0, 0, 0, 0, 0]);
// const C_CEDILLA: u64 = u64::from_le_bytes([0xC3, 0xA7, 0, 0, 0, 0, 0, 0]);
const OI: u64 = byte_arr_to_nr(b"oi");
const VA: u64 = byte_arr_to_nr(b"va");
const VE: u64 = byte_arr_to_nr(b"ve");
const JA: u64 = byte_arr_to_nr(b"ja");
const JE: u64 = byte_arr_to_nr(b"je");
const NI: u64 = byte_arr_to_nr(b"ni");
const TA: u64 = byte_arr_to_nr(b"ta");
const TI: u64 = byte_arr_to_nr(b"ti");
const TE: u64 = byte_arr_to_nr(b"te");
const IM: u64 = byte_arr_to_nr(b"im");
const IN: u64 = byte_arr_to_nr(b"in");
const RA: u64 = byte_arr_to_nr(b"ra");
const RI: u64 = byte_arr_to_nr(b"ri");
const UR: u64 = byte_arr_to_nr(b"ur");

fn suffix_2byte(st: u64) -> Option<&'static [&'static [u8]]> {
    // only the last two elements of the string are passed
    let mat: &[&[u8]] = match st & LAST_2_BYTES {
        // this one needed explicit coercion
        E_DIAERESIS => &[b""],
        OI => &[b"oj", b"uaj"],
        VA | VE => &[b"j", b"e", b""], // + "e" per shtie? but really low
        JA | JE => &[b"j", b"", b"e"],
        NI => &[b"j"],
        TA | TI => &[b"j", b""],
        TE => &[b"j", b""],
        IM | IN => &[b""],
        RA | RI => &[b"j"],
        UR => &[b""], // kto me ë psh do kalohen te ato qe duan 3
        // karaktere
        _ => return None,
    };
    Some(mat)
}

const MË: u64 = byte_arr_to_nr("më".as_bytes());
const RË: u64 = byte_arr_to_nr("rë".as_bytes());
const TË: u64 = byte_arr_to_nr("të".as_bytes());
const NË: u64 = byte_arr_to_nr("në".as_bytes());
const ËM: u64 = byte_arr_to_nr("ëm".as_bytes());
const ËT: u64 = byte_arr_to_nr("ët".as_bytes());
const ËN: u64 = byte_arr_to_nr("ën".as_bytes());

const OVA: u64 = byte_arr_to_nr(b"ova");
const OVE: u64 = byte_arr_to_nr(b"ove");
const UAM: u64 = byte_arr_to_nr(b"uam");
const UAT: u64 = byte_arr_to_nr(b"uat");
const UAN: u64 = byte_arr_to_nr(b"uan");
const JTA: u64 = byte_arr_to_nr(b"jta");
const JTE: u64 = byte_arr_to_nr(b"jte");
const JTI: u64 = byte_arr_to_nr(b"jti");
const NIM: u64 = byte_arr_to_nr(b"nim");
const NIT: u64 = byte_arr_to_nr(b"nit");
const NIN: u64 = byte_arr_to_nr(b"nin");
const NTE: u64 = byte_arr_to_nr(b"nte");
const TUR: u64 = byte_arr_to_nr(b"tur");
const UAR: u64 = byte_arr_to_nr(b"uar");

fn suffix_3byte(st: u64) -> Option<&'static [&'static [u8]]> {
    let mat: &[&[u8]] = match st & LAST_3_BYTES {
        MË | TË | NË => &[b"j", b"e", b""], // + "e" per shtie? but really low
        RË => &[b"j"],
        ËM | ËT | ËN => &[b""], // kto me ë psh do kalohen te ato qe duan 3
        OVA | OVE | UAM | UAT | UAN | UAR => &[b"oj", b"uaj"],
        JTA | JTE | JTI => &[b"j"],
        NIM | NIT | NIN => &[b"j", b"", b"e"],
        NTE => &[b"j"],
        TUR => &[b"j", b""],
        _ => return None,
    };
    Some(mat)
}

const JMË: u64 = byte_arr_to_nr("jmë".as_bytes());
const JNË: u64 = byte_arr_to_nr("jnë".as_bytes());
const TËM: u64 = byte_arr_to_nr("tëm".as_bytes());
const TËT: u64 = byte_arr_to_nr("tët".as_bytes());
const TËN: u64 = byte_arr_to_nr("tën".as_bytes());

fn suffix_4byte(st: u64) -> Option<&'static [&'static [u8]]> {
    let mat: &[&[u8]] = match st & LAST_4_BYTES {
        JMË | JNË => &[b"j"],
        TËM | TËT | TËN => &[b"j", b""],
        _ => return None,
    };
    Some(mat)
}

const JTËM: u64 = byte_arr_to_nr("jtëm".as_bytes());
const JTËT: u64 = byte_arr_to_nr("jtët".as_bytes());
const JTËN: u64 = byte_arr_to_nr("jtën".as_bytes());
const JTUR: u64 = byte_arr_to_nr("jtur".as_bytes());

fn suffix_5byte(st: u64) -> Option<&'static [&'static [u8]]> {
    let mat: &[&[u8]] = match st {
        JTËM | JTËT | JTËN | JTUR => &[b"j"],
        _ => return None,
    };
    Some(mat)
}
