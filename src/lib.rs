#![forbid(unsafe_code)]

use std::{
    collections::VecDeque,
    io::{BufReader, Read, Seek},
};

/// The RAR signature is identical between v4 and v5, save for the last 1–2 bytes that designates
/// the version.
const SIGNATURE: &[u8] = &[0x52, 0x61, 0x72, 0x21, 0x1A, 0x07];

const MAX_SFX_SIZE: usize = 1000 * 1000;

#[derive(Debug)]
pub enum Version {
    Rar4,
    Rar5,
}

pub struct Rar<R>
where
    R: Read + Seek,
{
    source: BufReader<R>,
}

impl<R> Rar<R>
where
    R: Read + Seek,
{
    pub fn open(source: R) -> Self {
        let reader = BufReader::new(source);

        let mut rar = Self { source: reader };

        rar.read_to_signature();

        let mut next_two_bytes = [0u8; 2];
        rar.source.read_exact(&mut next_two_bytes).unwrap();
        let version = match (next_two_bytes[0], next_two_bytes[1]) {
            (0x01, 0x00) => Version::Rar5,
            (0x00, _) => Version::Rar4,
            _ => panic!("Invalid version"),
        };
        dbg!(version);

        rar
    }

    /// Read through the source until either the signature is found, or we hit the maximum size of
    /// the SFX header (1MB).
    fn read_to_signature(&mut self) {
        let mut buffer = [0u8; 6];
        self.source.read_exact(&mut buffer).unwrap();

        let mut queue: VecDeque<u8> = VecDeque::with_capacity(6);
        queue.extend(buffer);

        // Read one byte at a time into a 6-byte long queue
        // If the queue contains the signature: go forward by one to get the version
        // Otherwise: pop byte off the left and add another one on the right
        let mut next_byte = [0u8; 1];
        let mut bytes_read = 0;
        while queue != SIGNATURE {
            self.source.read_exact(&mut next_byte).unwrap();
            queue.push_back(next_byte[0]);
            queue.pop_front();

            bytes_read += 1;

            if bytes_read > MAX_SFX_SIZE {
                panic!("Exceeded max SFX size. This isn't a RAR file");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::File, path::PathBuf};

    use crate::Rar;

    fn data(name: &str) -> File {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/data")
            .join(name);

        File::open(path).unwrap()
    }

    #[test]
    fn test_read_header() {
        Rar::open(data("rar5_stored.rar"));
    }

    #[test]
    fn test_sfx() {
        Rar::open(data("rar5_sfx.exe"));
    }
}
