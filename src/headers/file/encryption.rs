use bitflags::bitflags;

use crate::cursor::ByteCursor;

/// See <https://www.rarlab.com/technote.htm#filehead:~:text=File%20encryption%20record,-This>.
#[derive(Debug, Clone, PartialEq)]
pub struct EncryptionRecord {
    flags: Flags,
    kdf_count: u8,
    salt: [u8; 16],
    init_vec: [u8; 16],
    check_value: Option<[u8; 12]>,
}

impl EncryptionRecord {
    pub fn parse(cursor: &mut ByteCursor) -> Self {
        // There's currently only one supported value, 0, which corresponds to AES-256.
        let _version = cursor.read_vint().unwrap();

        let flags = Flags::from_bits_retain(cursor.read_vint().unwrap());

        let kdf_count = cursor.read_byte().unwrap();

        let salt = cursor.read_chunk::<16>().unwrap();

        let init_vec = cursor.read_chunk::<16>().unwrap();

        let check_value = if flags.contains(Flags::TWEAKED_CHECKSUMS) {
            Some(cursor.read_chunk::<12>().unwrap())
        } else {
            None
        };

        Self {
            flags,
            kdf_count,
            salt,
            init_vec,
            check_value,
        }
    }
}

bitflags! {
    /// See <https://www.rarlab.com/technote.htm#filehead:~:text=supported%2E-,Flags>.
    #[derive(Debug, Clone, PartialEq)]
    pub struct Flags: u64 {
        /// Password check data is present.
        const PASSWORD_CHECK = 0x0001;
        /// Use tweaked instead of plain checksums.
        /// See <https://www.rarlab.com/technote.htm#filehead:~:text=checksums%2E-,If>.
        const TWEAKED_CHECKSUMS = 0x0002;
    }
}
