use std::io::{Read, Seek};

/// Header data common to all headers
///
/// See <https://www.rarlab.com/technote.htm#genblock> for details.
pub struct BlockHeader {
    bytes: Vec<u8>,
}

impl BlockHeader {
    pub fn from_source<R>(mut source: R) -> Self
    where
        R: Read + Seek,
    {
        let mut crc_bytes = [0u8; 4];
        source.read_exact(&mut crc_bytes).unwrap();
        let stored_crc = u32::from_le_bytes(crc_bytes);

        let mut header_size_bytes = Vec::with_capacity(3);
        let mut header_size: u64 = 0;
        let mut shift = 0;
        // The header size field must not be longer than 3 bytes in the current implementation of RAR5
        for _ in 0..3 {
            let mut buffer = [0u8; 1];
            source.read_exact(&mut buffer).unwrap();
            let byte = buffer.first().unwrap();

            // The lower 7 bits contain the integer data
            let data = byte & 0x7F;

            // We need both the raw bytes and the u64 value. The first for checking the CRC, the
            // second for knowing how many bytes to buffer for the header.
            header_size_bytes.push(*byte);
            header_size |= (data as u64) << shift;

            // The highest bit contains the continue flag
            if byte & 0x80 != 0 {
                shift += 7;
                continue;
            } else {
                break;
            }
        }

        assert!(header_size <= 2 * 1000 * 1000);

        let mut buffer = vec![0u8; header_size.try_into().unwrap()];
        source.read_exact(&mut buffer).unwrap();

        // The CRC is of everything from the header size field up to and including the optional extra area
        let mut bytes = header_size_bytes;
        bytes.extend(&buffer);

        let computed_crc = crc32fast::hash(&bytes);

        println!("Stored: {:02x?}", stored_crc);
        println!("Computed: {:02x?}", computed_crc);

        assert_eq!(stored_crc, computed_crc);

        Self { bytes: buffer }
    }
}

#[cfg(test)]
mod test {
    use std::{
        fs::File,
        io::{Seek, SeekFrom},
        path::PathBuf,
    };

    use crate::block::BlockHeader;

    fn data(name: &str) -> File {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/data")
            .join(name);

        File::open(path).unwrap()
    }

    #[test]
    fn rar5_stored() {
        let mut source = data("rar5_stored.rar");

        // Skip the signature
        source.seek(SeekFrom::Start(8)).unwrap();

        BlockHeader::from_source(source);
    }
}
