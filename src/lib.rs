#![forbid(unsafe_code)]

use std::io::Read;

const RAR5_SIGNATURE: &[u8] = &[0x52, 0x61, 0x72, 0x21, 0x1A, 0x07, 0x01, 0x00];

pub struct Rar {}

impl Rar {
    pub fn open<R: Read>(mut source: R) {
        let mut buffer: Vec<u8> = Vec::new();

        source.read_to_end(&mut buffer).unwrap();

        // Assumes this isn't an SFX archive
        assert_eq!(&buffer[..8], RAR5_SIGNATURE);
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
}
