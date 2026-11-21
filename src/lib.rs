#![forbid(unsafe_code)]

use std::{
    collections::VecDeque,
    io::{BufReader, Read, Seek},
};

use crate::{
    block::Block,
    headers::{Header, main::MainHeader},
};

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
    reader: BufReader<R>,
    main_header: MainHeader,
}

impl<R> Archive<R>
where
    R: Read + Seek,
{
    pub fn open(source: R) -> Self {
        let mut reader = BufReader::new(source);

        Self::read_to_signature(&mut reader);

        let mut next_byte = [0u8; 1];
        reader.read_exact(&mut next_byte).unwrap();
        let version = match next_byte[0] {
            0x01 => {
                reader.read_exact(&mut next_byte).unwrap();
                if next_byte[0] == 0x00 {
                    Version::Rar5
                } else {
                    panic!("Invalid version");
                }
            }
            0x00 => Version::Rar4,
            _ => panic!("Invalid version"),
        };

        // TODO: Assumes there isn't an encryption header

        let main_header = if let Header::Main(header) = Block::next(&mut reader).header {
            header
        } else {
            panic!("No main header");
        };

        // TODO: Assumes there isn't an archive comment service header

        Self {
            reader,
            main_header,
        }
    }

    pub fn next_block<'a>(&'a mut self) -> Block<'a, R> {
        Block::next(&mut self.reader)
    }

    /// Read through the source until either the signature is found, or we hit the maximum size of
    /// the SFX header (1MB).
    fn read_to_signature(source: &mut BufReader<R>) {
        let mut buffer = [0u8; 6];
        source.read_exact(&mut buffer).unwrap();

        let mut queue: VecDeque<u8> = VecDeque::with_capacity(6);
        queue.extend(buffer);

        // Read one byte at a time into a 6-byte long queue
        // If the queue contains the signature: go forward by one to get the version
        // Otherwise: pop byte off the left and add another one on the right
        let mut next_byte = [0u8; 1];
        let mut bytes_read = 0;
        while queue != SIGNATURE {
            source.read_exact(&mut next_byte).unwrap();
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
        use std::{fs::File, path::PathBuf};

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
        Archive::open_test("libarchive/v5/stored.rar");
    }

    #[test]
    fn sfx() {
        Archive::open_test("libarchive/v5/sfx.exe");
    }
}
