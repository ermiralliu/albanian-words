use std::{
    fs::File,
    io::{BufRead, BufReader},
};

pub struct SequentialFileReader {
    reader: BufReader<File>,
    delimiter: u8,
}

impl SequentialFileReader {
    pub fn try_new(filepath: &str, delimiter: u8) -> Result<SequentialFileReader, std::io::Error> {
        let file = File::open(filepath)?;
        let reader = BufReader::new(file);
        Ok(SequentialFileReader {
            reader,
            delimiter,
        })
    }

    // These are parts that I'm adding so I can make the logic more reusable
    pub fn read_into(&mut self, buf: &mut Vec<u8>) -> bool {
        let Ok(bytes_read) = self.reader.read_until(self.delimiter, buf) else { return false };

        // If no bytes read, we've reached EOF
        if bytes_read == 0 {
            return false;
        }
        // buf.truncate(buf.len()-1);

        return true;
    }
}
