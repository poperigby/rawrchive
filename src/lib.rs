#![forbid(unsafe_code)]

#[cfg(test)]
use std::fs::File;
use std::{
    collections::VecDeque,
    io::{BufReader, Read, Seek},
    path::PathBuf,
};

use crate::block::Block;

mod block;
mod cursor;
mod headers;

/// The RAR signature is identical between v4 and v5, save for the last 1–2 bytes that designates
/// the version.
const SIGNATURE: &[u8] = &[0x52, 0x61, 0x72, 0x21, 0x1A, 0x07];

const SFX_MAX_SIZE: usize = 1000 * 1000;

#[derive(Debug)]
pub enum Version {
    Rar4,
    Rar5,
}

pub struct Archive<R>
where
    R: Read + Seek,
{
    source: BufReader<R>,
}

impl<R> Archive<R>
where
    R: Read + Seek,
{
    pub fn open(source: R) -> Self {
        let reader = BufReader::new(source);

        let mut a = Self { source: reader };

        a.read_to_signature();

        let mut next_byte = [0u8; 1];
        a.source.read_exact(&mut next_byte).unwrap();
        let version = match next_byte[0] {
            0x01 => {
                a.source.read_exact(&mut next_byte).unwrap();
                if next_byte[0] == 0x00 {
                    Version::Rar5
                } else {
                    panic!("Invalid version");
                }
            }
            0x00 => Version::Rar4,
            _ => panic!("Invalid version"),
        };

        // TODO: Parse through the opening blocks until we get to the file blocks

        a
    }

    pub fn next_block<'a>(&'a mut self) -> Block<'a, R> {
        Block::new(&mut self.source)
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

            if bytes_read > SFX_MAX_SIZE {
                panic!("Exceeded max SFX size. This isn't a RAR file");
            }
        }
    }
}

#[cfg(test)]
impl Archive<std::fs::File> {
    pub(crate) fn open_test(name: &str) -> Self {
        use std::fs::File;

        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/data")
            .join(name);

        let file = File::open(path).unwrap();

        Archive::open(file)
    }
}

#[cfg(test)]
mod tests {
    use crate::Archive;

    #[test]
    fn stored() {
        Archive::open_test("rar5_stored.rar");
    }

    #[test]
    fn sfx() {
        Archive::open_test("rar5_sfx.exe");
    }
}
