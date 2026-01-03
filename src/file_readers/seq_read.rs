use std::{
    fs::File,
    io::{BufRead, BufReader},
};

pub struct SequentialFileReader {
    reader: BufReader<File>,
    // buffer: Vec<u8>,
    delimiter: u8,
}

impl SequentialFileReader {
    pub fn try_new(filepath: &str, delimiter: u8) -> Result<SequentialFileReader, std::io::Error> {
        let file = File::open(filepath)?;
        let mut reader = BufReader::new(file);
        // let mut buffer = Vec::with_capacity(1024 * 256); // 256 kB
        Ok(SequentialFileReader {
            reader,
            // buffer,
            delimiter,
        })
    }

    // fn read_and_process<T>(&mut self, process_line: fn(&str) -> T) {
    //     loop {
    //         self.buffer.clear();
    //         // handle the error below
    //         let Ok(bytes_read) = self.reader.read_until(self.delimiter, &mut self.buffer) else { continue };
    //
    //         // If no bytes read, we've reached EOF
    //         if bytes_read == 0 {
    //             break;
    //         }
    //
    //         // Convert to string and process
    //         let Ok(line) = std::str::from_utf8(&self.buffer) else { continue };
    //         // I would have wanted to use the unchecked version ngl
    //
    //         // let line = line.trim(); // Remove newline and whitespace
    //
    //         println!("Processed line: {}", line);
    //
    //         // Your processing logic here
    //         process_line(line);
    //     }
    // }
    // These are parts that I'm adding so I can make the logic more reusable
    pub fn read_into(&mut self, buf: &mut Vec<u8>) -> bool {
        let Ok(bytes_read) = self.reader.read_until(self.delimiter, buf) else { return false };

        // If no bytes read, we've reached EOF
        if bytes_read == 0 {
            return false;
        }
        buf.truncate(buf.len()-1);

        return true;
    }
}
