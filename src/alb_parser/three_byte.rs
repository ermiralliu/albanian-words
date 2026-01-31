pub const JE_EM_ARR: &[&[u8]] = &[b"j", b"e", b""];
pub const J_ARR: &[&[u8]] = &[b"j"];
pub const J_EM_ARR: &[&[u8]] = &[b"j", b""];
pub const EMPTY_ARR: &[&[u8]] = &[b""];
pub const OJ_UAJ_ARR: &[&[u8]] = &[b"oj", b"uaj"];

// use std::sync::atomic::Ordering;

use crate::alb_parser::*;

const SUFFIXES: &[u64] = &[
    MË, RË, TË, NË, ËM, ËT, ËN, OVA, OVE, UAM, UAT, UAN, JTA, JTE, JTI, NIM, NIT, NIN, NTE, TUR, UAR,
];

const MAGIC_MUL: u64 = 481769;

// pub const SUFFIX_LOOKUP: &[Option<(u64, &[&[u8]])>; 32] = &{
//     let mut i = 0;
//     let mut arr: [Option<(u64, &[&[u8]])>; 32] = [None; 32];
//     loop {
//         let index = ((SUFFIXES[i].wrapping_mul(481769) >> 20) % 32) as usize;
//         let mat: &[&[u8]] = match SUFFIXES[i] {
//             MË | TË | NË => JE_EM_ARR, // + "e" per shtie? but really low
//             RË => J_ARR,
//             ËM | ËT | ËN => EMPTY_ARR, // kto me ë psh do kalohen te ato qe duan 3
//             OVA | OVE | UAM | UAT | UAN | UAR => OJ_UAJ_ARR,
//             JTA | JTE | JTI => J_ARR,
//             NIM | NIT | NIN => JE_EM_ARR,
//             NTE => J_ARR,
//             TUR => J_EM_ARR,
//             _ => {
//                 i += 1;
//                 if i == SUFFIXES.len() {
//                     break;
//                 };
//                 continue;
//             }
//         };
//
//         arr[index] = Some((SUFFIXES[i], mat));
//         i += 1;
//         if i == SUFFIXES.len() {
//             break;
//         };
//     }
//     arr
// };
//
// // #[inline]
// // #[unsafe(no_mangle)]
// pub fn suffix_3byte_final(st: u64) -> Option<&'static [&'static [u8]]> {
//     let index = st.wrapping_mul(MAGIC_MUL) >> 20;
//     let index = index % 32;
//     let end = SUFFIX_LOOKUP[index as usize];
//     let Some(value) = end else { return None };
//     if st != value.0 {
//         return None;
//     }
//     Some(value.1)
// }

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum ResultOf3Byte {
    None,
    EmptyArr,
    JeEmArr,
    JArr,
    JEmArr,
    OjUajArr,
}

#[derive(Debug, Clone, Copy)]
pub struct LookupTable {
    pub keys: [u32; 32],
    pub values: [ResultOf3Byte; 32],
}

pub const SUFFIX_LOOKUP: LookupTable = {
    let mut i = 0;
    let mut arr: [ResultOf3Byte; 32] = [ResultOf3Byte::None; 32];
    let mut arr_keys = [0u32; 32];
    loop {
        let index = ((SUFFIXES[i].wrapping_mul(481769) >> 20) % 32) as usize;
        let mat: ResultOf3Byte = match SUFFIXES[i] {
            MË | TË | NË => ResultOf3Byte::JEmArr, // + "e" per shtie? but really low
            RË => ResultOf3Byte::JArr,
            ËM | ËT | ËN => ResultOf3Byte::EmptyArr, // kto me ë psh do kalohen te ato qe duan 3
            OVA | OVE | UAM | UAT | UAN | UAR => ResultOf3Byte::OjUajArr,
            JTA | JTE | JTI => ResultOf3Byte::JArr,
            NIM | NIT | NIN => ResultOf3Byte::JeEmArr,
            NTE => ResultOf3Byte::JArr,
            TUR => ResultOf3Byte::JEmArr,
            _ => ResultOf3Byte::None,
        };

        arr_keys[index] = SUFFIXES[i] as u32;
        arr[index] = mat;
        i += 1;
        if i == SUFFIXES.len() {
            break;
        };
    }
    LookupTable {
        keys: arr_keys,
        values: arr,
    }
};

// #[inline]
// #[unsafe(no_mangle)]
pub fn suffix_3byte_final(st: u64) -> Option<&'static [&'static [u8]]> {
    // CALL_COUNT_3.fetch_add(1, Ordering::Relaxed);
    const LAST_3_BYTES: u64 = 0x0000_0000_00FF_FFFF;
    let st = st & LAST_3_BYTES;
    let index = st.wrapping_mul(MAGIC_MUL) >> 20;
    let index = index % 32;
    let index = index as usize;
    if st as u32 != SUFFIX_LOOKUP.keys[index] {
        return Option::None;
    }
    let end = SUFFIX_LOOKUP.values[index];
    let value = match end {
        ResultOf3Byte::None => return Option::None,
        ResultOf3Byte::EmptyArr => EMPTY_ARR,
        ResultOf3Byte::JeEmArr => JE_EM_ARR,
        ResultOf3Byte::JArr => J_ARR,
        ResultOf3Byte::JEmArr => J_EM_ARR,
        ResultOf3Byte::OjUajArr => OJ_UAJ_ARR,
    };
    Some(value)
}
