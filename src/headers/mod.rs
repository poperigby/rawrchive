use bitflags::bitflags;

use crate::{
    cursor::ByteCursor,
    headers::{file::FileHeader, main::MainHeader},
};

mod file;
mod main;

#[derive(Debug, Clone, PartialEq)]
pub enum Header {
    Main(MainHeader),
    File(FileHeader),
    Service,
    Encryption,
    End,
}

impl Header {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut cursor = ByteCursor::from_bytes(bytes);

        let kind = cursor.read_vint().unwrap();
        let flags = HeaderFlags::from_bits_retain(cursor.read_vint().unwrap());

        match kind {
            1 => Self::Main(MainHeader::parse(&mut cursor, flags)),
            2 => Self::File(FileHeader::parse(&mut cursor, flags)),
            3 => Self::Service,
            4 => Self::Encryption,
            5 => Self::End,
            _ => panic!("Unknown header type"),
        }
    }
}

bitflags! {
    /// See `Header flags` field in <https://www.rarlab.com/technote.htm#genblock>
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct HeaderFlags: u64 {
        /// Header is followed by an extra area.
        const EXTRA_AREA = 0x0001;
        /// Header is followed by a data area.
        const DATA_AREA = 0x0002;
        /// Skip blocks of unknown type with this flag when updating the archive.
        const SKIP_IF_UNKNOWN = 0x0004;
        /// Data area continues from the previous volume.
        const SPLIT_BEFORE = 0x0008;
        /// Data area continues in the next volume.
        const SPLIT_AFTER = 0x0010;
        /// Block depends on the preceding file block.
        const DEPENDS_ON_PREV = 0x0020;
        /// Keep child block when the parent block is modified.
        const PRESERVE_CHILD = 0x0040;
    }
}
