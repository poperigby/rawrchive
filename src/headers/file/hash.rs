use crate::cursor::ByteCursor;

/// See <https://www.rarlab.com/technote.htm#filehead:~:text=File%20hash%20record,-Only>
#[derive(Debug, Clone, PartialEq)]
pub struct HashRecord {
    data: Vec<u8>,
}

impl HashRecord {
    pub fn parse(cursor: &mut ByteCursor) -> Self {
        // There's currently only one supported value, 0, which corresponds to BLAKE2sp.
        let hash_type = cursor.read_vint().unwrap();

        let data_size = match hash_type {
            0x00 => 32,
            _ => panic!("Unknown hash type"),
        };

        let data = cursor.read_bytes(data_size).unwrap().to_vec();

        Self { data }
    }
}
