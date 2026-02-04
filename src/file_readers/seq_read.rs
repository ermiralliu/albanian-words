use std::{
    fs::File,
    io::{BufRead, BufReader},
};

pub struct SequentialFileReader {
    reader: BufReader<File>,
    delimiter: u8,
}

const LANESIZE: usize = 64;

impl SequentialFileReader {
    pub fn try_new(filepath: &str, delimiter: u8) -> Result<SequentialFileReader, std::io::Error> {
        let file = File::open(filepath)?;
        let reader = BufReader::new(file);
        Ok(SequentialFileReader { reader, delimiter })
    }

    // These are parts that I'm adding so I can make the logic more reusable
    pub fn read_into(&mut self, buf: &mut Vec<u8>) -> bool {
        let Ok(bytes_read) = self.reader.read_until(self.delimiter, buf) else { return false };

        // If no bytes read, we've reached EOF
        if bytes_read == 0 {
            return false;
        }
        // buf.truncate(buf.len()-1);
        let remainder = buf.len() % LANESIZE; // this was % SIMD_BYTESIZE at first
        if remainder != 0 {
            let padding_needed = LANESIZE - remainder;

            // Ensure we have space for the padding.
            // If len + padding > capacity, this handles the growth.
            if buf.capacity() - buf.len() < padding_needed {
                buf.reserve(padding_needed);
            }

            buf.extend(std::iter::repeat(0).take(padding_needed));
        }

        return true;
    }
}
