use std::{fs::File, io::Read};


pub struct FileData {
    buffer: Vec<u8>,
    delimiter_indices: Vec<usize>,
}
impl FileData {
    pub fn try_from(filename: &str, delimiter: u8) -> Result<FileData, std::io::Error> {
        let mut file = File::open(filename)?;
        let buffer: Vec<u8> = {
            let mut yeah: Vec<u8> = Vec::new();
            file.read_to_end(&mut yeah)?; // Here we are simply reading the whole file, which
            // is why we later continue by simply checking for
            // the delimiter. Normally we would both fill the
            // buffer and check for the delimiter, which means,
            // stuff is not so simple.
            yeah
        };
        let buff_slice = &buffer[..];

        let mut delimiter_indices = Vec::new();

        for pos in memchr::memchr_iter(delimiter, buff_slice) {
            delimiter_indices.push(pos);
        }
        if delimiter_indices[delimiter_indices.len() - 1] != buffer.len() {
            delimiter_indices.push(buffer.len());
        }

        Ok(FileData {
            buffer,
            delimiter_indices,
        })
    }

    pub fn chunks(&self) -> impl Iterator<Item = &[u8]> { // use this as a basis and be careful.
        // The 'end' variable IS the index of the delimiter for the current chunk.
        self.delimiter_indices.iter().scan(0, move |last_pos, &end| {
            // Current chunk ends at 'end'
            let ret_slice = &self.buffer[*last_pos..end];

            // Start of the NEXT chunk (after the delimiter)
            *last_pos = end + 1;

            Some(ret_slice)
        })
    }
    //
    // pub fn get(&self, index: usize) -> Option<&[u8]> {
    //     self.delimiter_indices
    //         .get(index)
    //         .map(|(start, end)| &self.buffer[*start..*end])
    // }

    pub fn len(&self) -> usize {
        self.delimiter_indices.len()
    }
}
