use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;
// use quote::{format_ident, quote};

fn main() {
    let out_dir = "data";
    let vocab_binary = Path::new("data/vocab.bin"); // I actually don't need to print these
    let vocab_offsets = Path::new("data/vocab_offsets.bin");

    let mut vbf = File::create(&vocab_binary).unwrap();
    let mut vof = File::create(&vocab_offsets).unwrap();


    let mut vocab_content =
        fs::read_to_string("./new_sorted_vocab.txt").expect("where is it?");
    let mut indices: Vec<u32> = std::iter::once(0)
        .chain(vocab_content.match_indices('\n').map(|(i, _)| i as u32))
        .collect();
    let mut iterator = indices.iter_mut().enumerate();
    iterator.next();
    iterator.for_each(|(i, x)| *x = *x - i as u32 + 1);

    vocab_content.retain(|c| c != '\n');
    let mut vocab_content = vocab_content.into_bytes();
    vocab_content.iter_mut().for_each(|x| {
        if *x != 0xC3 {
            *x |= 0x20
        }
    });
    let _ = vbf.write_all(&vocab_content[..]);
    for &num in &indices {
        let _ = vof.write_all(&num.to_le_bytes());
    }

    let mut words_tuple = Vec::new();
    for (id, windows) in indices.windows(2).enumerate() {
        let &[offset_start, offset_end] = windows else {
            break;
        };
        words_tuple.push((
            &vocab_content[offset_start as usize..offset_end as usize],
            id as u64,
        ));
    } // 2. Build the FST in a memory buffer (Vec<u8>)
    words_tuple.sort();

    // 2. Build the FST
    let dest_path = Path::new(&out_dir).join("dictionary.bin");
    let fst_file = BufWriter::new(File::create(&dest_path).unwrap());
    let mut builder = fst::MapBuilder::new(fst_file).unwrap();
    for (word, id) in words_tuple {
        builder.insert(word, id).unwrap();
    }
    builder.finish().unwrap();
    
    println!("cargo:rerun-if-changed=build.rs");
}
