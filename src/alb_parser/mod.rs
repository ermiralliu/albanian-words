use std::{collections::HashMap, hash::BuildHasher, hint::unreachable_unchecked};

use crate::alb_parser::{one_byte::suffix_1byte_new, three_byte::{J_EM_ARR, suffix_3byte_final}};
pub mod three_byte;
pub mod one_byte;
// use unicode_normalization::UnicodeNormalization;

// type SuffixFunction = fn(&str) -> Option<&[&str]>;
type SuffixFunction = fn(u64) -> Option<&'static [&'static [u8]]>;

// using &[u8] lets you have fun without worrying about sizes. Nice Rust stuff.

const DEFAULT_WORD_BUFFER_CAPACITY: usize = 32; // Increased this size only because of some
// retarded articles

pub struct AlbanianParser<'a, K>
where
    K: BuildHasher,
{
    vocab: &'a HashMap<&'a [u8], u16, K>,
    // normalization_buffer: String, // after normalizing ë and ç
    // main_buffer: String,          // after lowercasing the normalization buffer
    base_form: [u8; DEFAULT_WORD_BUFFER_CAPACITY], // after lowercasing the normalization buffer
    base_form_len: usize,
}

impl<'a, K> AlbanianParser<'a, K>
where
    K: BuildHasher + Default,
{
    pub fn new(vocab: &'a HashMap<&'a [u8], u16, K>) -> AlbanianParser<'a, K> {
        AlbanianParser {
            vocab,
            base_form: [0u8; DEFAULT_WORD_BUFFER_CAPACITY],
            base_form_len: 0,
        }
    }

    #[inline]
    pub fn single_verb_to_base(&mut self, verb: &[u8]) -> Option<u16> {
        // we can use copy non-overlapping if the copy below is not enough
        if verb.len() >= 32 {
            unsafe { std::hint::unreachable_unchecked() };
            // I was wondering how to deal with it but yeah. Just return nothing.
            // The largest albanian word is less
            // return None;
        }
        self.base_form[..verb.len()].copy_from_slice(verb);
        let len = verb.len();
        self.base_form_len = len;

        const CHECKS: &[SuffixFunction] = &[
            // suffix_1byte,
            suffix_1byte_new,
            suffix_2byte,
            // suffix_3byte_new,
            suffix_3byte_final,
            suffix_4byte,
            suffix_5byte,
        ];

        let range = match len {
            0..=2 => unsafe { std::hint::unreachable_unchecked() }, // we're checking outside the function
            3..=6 => 2..len,
            7.. => len - 5..len,
        };
        let range_length = (&range).len();

        let initial_suffix = byte_arr_to_nr_clean(&verb[range]);

        CHECKS[..range_length.min(5)]
            .iter()
            .enumerate()
            .rev() // Iterate from smallest to largest suffix (or vice versa depending on array order)
            .find_map(|(i, &func)| self.possibilities_for_verb(verb, initial_suffix, i + 1, func))
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
// const LAST_4_BYTES: u64 = 0xFFFF_FFFF_0000_0000;
// const LAST_3_BYTES: u64 = 0xFFFF_FF00_0000_0000;
// const LAST_2_BYTES: u64 = 0xFFFF_0000_0000_0000;
// const LAST_BYTE: u64 = 0xFF00_0000_0000_0000;

const LAST_4_BYTES: u64 = 0x0000_0000_FFFF_FFFF;
// const LAST_3_BYTES: u64 = 0x0000_0000_00FF_FFFF;
const LAST_2_BYTES: u64 = 0x0000_0000_0000_FFFF;
const LAST_BYTE: u64 = 0x0000_0000_0000_00FF;

const fn byte_arr_to_nr(st: &[u8]) -> u64 {
    let mut s = 0u64; // Initialize 8 bytes of zeros
    let len = st.len();
    let dest_start = unsafe { (&mut s as *mut u64 as *mut u8).add(8 - len) };

    unsafe {
        // This is a direct raw pointer copy (memcpy)
        std::ptr::copy_nonoverlapping(st.as_ptr(), dest_start, len);
    }
    u64::from_be(s)
}
// #[unsafe(no_mangle)]
fn byte_arr_to_nr_clean(st: &[u8]) -> u64 {
    let len = st.len();

    // Force 8-byte load
    let raw = unsafe { (st.as_ptr() as *const u64).read_unaligned() };
    // Flip to Big Endian so the first byte is the most significant
    let big_endian = raw.swap_bytes();

    big_endian >> (64 - (len * 8))
}

const U: u64 = byte_arr_to_nr(b"u");
const J: u64 = byte_arr_to_nr(b"j");
const N: u64 = byte_arr_to_nr(b"n");
const A: u64 = byte_arr_to_nr(b"a");
const E: u64 = byte_arr_to_nr(b"e");
const I: u64 = byte_arr_to_nr(b"i");

fn suffix_1byte(ch: u64) -> Option<&'static [&'static [u8]]> {
    // CALL_COUNT_1.fetch_add(1, Ordering::Relaxed);
    // For now, I'm keeping it simple with
    // static lifetimes
    let j_em_arr = J_EM_ARR; // &[b"j", b"e", b""];
    let final_byte = ch & LAST_BYTE;
    match final_byte % 4 {
        0 if matches!(final_byte, J | N) => {
            return Some(unsafe { j_em_arr.get_unchecked(2..=2)});
        }
        1 => {
            if final_byte >= 97 && final_byte <= 105 {
                return Some(unsafe { j_em_arr.get_unchecked(2..=2)});
            } else if final_byte == U {
                return Some(j_em_arr);
            }
        }
        2 | 3 => return None,
        _ => unsafe { unreachable_unchecked() },
    }
    None
    // if final_byte % 4 == 0 && matches!(final_byte, J|N) {
    //     return Some(&j_em_arr[0..=0]);
    // }
    // let mat: &[&[u8]] = match final_byte {
    //     U => j_em_arr,
    //     J | N => &j_em_arr[0..=0],
    //     A | E | I => &j_em_arr[2..=2],
    //     _ => return None,
    // };
    // Some(mat)
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
    // CALL_COUNT_2.fetch_add(1, Ordering::Relaxed);
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

pub const MË: u64 = byte_arr_to_nr("më".as_bytes());
pub const RË: u64 = byte_arr_to_nr("rë".as_bytes());
pub const TË: u64 = byte_arr_to_nr("të".as_bytes());
pub const NË: u64 = byte_arr_to_nr("në".as_bytes());
pub const ËM: u64 = byte_arr_to_nr("ëm".as_bytes());
pub const ËT: u64 = byte_arr_to_nr("ët".as_bytes());
pub const ËN: u64 = byte_arr_to_nr("ën".as_bytes());

pub const OVA: u64 = byte_arr_to_nr(b"ova");
pub const OVE: u64 = byte_arr_to_nr(b"ove");
pub const UAM: u64 = byte_arr_to_nr(b"uam");
pub const UAT: u64 = byte_arr_to_nr(b"uat");
pub const UAN: u64 = byte_arr_to_nr(b"uan");
pub const JTA: u64 = byte_arr_to_nr(b"jta");
pub const JTE: u64 = byte_arr_to_nr(b"jte");
pub const JTI: u64 = byte_arr_to_nr(b"jti");
pub const NIM: u64 = byte_arr_to_nr(b"nim");
pub const NIT: u64 = byte_arr_to_nr(b"nit");
pub const NIN: u64 = byte_arr_to_nr(b"nin");
pub const NTE: u64 = byte_arr_to_nr(b"nte");
pub const TUR: u64 = byte_arr_to_nr(b"tur");
pub const UAR: u64 = byte_arr_to_nr(b"uar");

// fn suffix_3byte(st: u64) -> Option<&'static [&'static [u8]]> {
//     let mat: &[&[u8]] = match st & LAST_3_BYTES {
//         MË | TË | NË => &[b"j", b"e", b""], // + "e" per shtie? but really low
//         RË => &[b"j"],
//         ËM | ËT | ËN => &[b""], // kto me ë psh do kalohen te ato qe duan 3
//         OVA | OVE | UAM | UAT | UAN | UAR => &[b"oj", b"uaj"],
//         JTA | JTE | JTI => &[b"j"],
//         NIM | NIT | NIN => &[b"j", b"", b"e"],
//         NTE => &[b"j"],
//         TUR => &[b"j", b""],
//         _ => return None,
//     };
//     Some(mat)
// }

const JMË: u64 = byte_arr_to_nr("jmë".as_bytes());
const JNË: u64 = byte_arr_to_nr("jnë".as_bytes());
const TËM: u64 = byte_arr_to_nr("tëm".as_bytes());
const TËT: u64 = byte_arr_to_nr("tët".as_bytes());
const TËN: u64 = byte_arr_to_nr("tën".as_bytes());

fn suffix_4byte(st: u64) -> Option<&'static [&'static [u8]]> {
    // CALL_COUNT_4.fetch_add(1, Ordering::Relaxed);
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
    // CALL_COUNT_5.fetch_add(1, Ordering::Relaxed);
    let mat: &[&[u8]] = match st {
        JTËM | JTËT | JTËN | JTUR => &[b"j"],
        _ => return None,
    };
    Some(mat)
}
