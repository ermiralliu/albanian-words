use crate::alb_parser::{LAST_BYTE, byte_arr_to_nr, three_byte::{EMPTY_ARR, J_ARR, J_EM_ARR, JE_EM_ARR}};

const U: usize = byte_arr_to_nr(b"u") as usize;
const J: usize = byte_arr_to_nr(b"j") as usize;
const N: usize = byte_arr_to_nr(b"n") as usize;
const A: usize = byte_arr_to_nr(b"a") as usize;
const E: usize = byte_arr_to_nr(b"e") as usize;
const I: usize = byte_arr_to_nr(b"i") as usize;

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
enum OneByteOptions {
    None = 0,
    JEmArr = 1,
    JArr = 2,
    EmptyArr= 3
}

const ONE_BYTE_LOOKUP: [OneByteOptions; 32] = {
    let mut arr = [OneByteOptions::None; 32];
    const FLOOR: usize = 97;
    arr[U-FLOOR] = OneByteOptions::JEmArr;
    arr[J-FLOOR] = OneByteOptions::JArr;
    arr[N-FLOOR] = OneByteOptions::JArr;
    arr[A-FLOOR] = OneByteOptions::EmptyArr;
    arr[E-FLOOR] = OneByteOptions::EmptyArr;
    arr[I-FLOOR] = OneByteOptions::EmptyArr;
    arr
};

const ONE_BYTE_LOOKUP_BITSET: u64 = {
    const FLOOR: usize = 97;
    let jemarray = (OneByteOptions::JEmArr as u64) << U - FLOOR;
    let j = (OneByteOptions::JArr as u64) << J - FLOOR;
    let n = (OneByteOptions::JArr as u64) << N - FLOOR; 
    let a = (OneByteOptions::EmptyArr as u64) << A - FLOOR;
    let e = (OneByteOptions::EmptyArr as u64) << E - FLOOR; 
    let i = (OneByteOptions::EmptyArr as u64) << I - FLOOR; 
    jemarray | j | n | a | e | i
};

// #[unsafe(no_mangle)]
// pub fn suffix_1byte_new(ch: u64) -> Option<&'static [&'static [u8]]> {
//     let final_byte = ch & LAST_BYTE;
//     if final_byte > 117 || final_byte < 97 {
//         return None;
//     }
//
//     let index: usize = (final_byte - (b'a' as u64)) as usize;
//     match unsafe { ONE_BYTE_LOOKUP.get_unchecked(index)} {
//         OneByteOptions::None => return Option::None,
//         OneByteOptions::JEmArr => return Some(J_EM_ARR),
//         OneByteOptions::JArr => return Some(J_ARR),
//         OneByteOptions::EmptyArr => return Some(EMPTY_ARR),
//     }
// }

pub fn suffix_1byte_new(ch: u64) -> Option<&'static [&'static [u8]]> {
    let final_byte = ch & LAST_BYTE;
    if final_byte > 117 || final_byte < 97 {
        return None;
    }

    let index: usize = (final_byte - (b'a' as u64)) as usize;
    let transmutation = unsafe { std::mem::transmute(((ONE_BYTE_LOOKUP_BITSET >> (index * 2)) & (4-1)) as u8) };
    match transmutation {
        OneByteOptions::None => return Option::None,
        OneByteOptions::JEmArr => return Some(J_EM_ARR),
        OneByteOptions::JArr => return Some(J_ARR),
        OneByteOptions::EmptyArr => return Some(EMPTY_ARR),
    }
}
