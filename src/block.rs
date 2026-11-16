use std::io::{BufReader, Read, Seek};

use bitflags::bitflags;

use crate::cursor::ByteCursor;

#[derive(Debug)]
pub struct Block<'a, R>
where
    R: Read + Seek,
{
    source: &'a mut BufReader<R>,
    header: Header,
}

impl<'a, R> Block<'a, R>
where
    R: Read + Seek,
{
    pub fn new(source: &'a mut BufReader<R>) -> Self {
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

        assert_eq!(stored_crc, computed_crc);

        let header = Header::from_bytes(&buffer);

        Self { source, header }
    }
}

#[derive(Debug, Clone)]
pub enum Header {
    Main(MainHeader),
    File,
    Service,
    Encryption,
    End,
}

impl Header {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut c = ByteCursor::from_bytes(bytes);

        let kind = c.read_vint().unwrap();
        let flags = HeaderFlags::from_bits_retain(c.read_vint().unwrap());

        match kind {
            1 => Self::Main(MainHeader::new(c, flags)),
            2 => Self::File,
            3 => Self::Service,
            4 => Self::Encryption,
            5 => Self::End,
            _ => panic!("Unknown header type"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MainHeader {
    extra_area_size: Option<u64>,
    archive_flags: ArchiveFlags,
    volume_number: Option<u64>,
}
impl MainHeader {
    fn new(mut cursor: ByteCursor, flags: HeaderFlags) -> Self {
        let extra_area_size = if flags.contains(HeaderFlags::EXTRA_AREA) {
            Some(cursor.read_vint().unwrap())
        } else {
            None
        };

        let archive_flags = ArchiveFlags::from_bits_retain(cursor.read_vint().unwrap());

        let volume_number = if archive_flags.contains(ArchiveFlags::MULTI_VOLUME) {
            Some(cursor.read_vint().unwrap())
        } else {
            None
        };

        Self {
            extra_area_size,
            archive_flags,
            volume_number,
        }
    }
}

/// See `Header flags` field in <https://www.rarlab.com/technote.htm#genblock>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HeaderFlags(u64);
bitflags! {
    impl HeaderFlags: u64 {
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

/// See `Archive flags` field in <https://www.rarlab.com/technote.htm#mainhead>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArchiveFlags(u64);
bitflags! {
    impl ArchiveFlags: u64 {
        /// This archive is part of a multi-volume set.
        const MULTI_VOLUME = 0x0001;
        /// The volumne number field is present. This flag is set in all but the first volume.
        const VOLUME_NUMBER = 0x0002;
        /// This is a solid archive: <https://en.wikipedia.org/wiki/Solid_compression>.
        const SOLID = 0x0004;
        /// A recovery record is present.
        const RECOVERY_RECORD = 0x0008;
        /// This archive is locked.
        const LOCKED = 0x0010;
    }
}

#[cfg(test)]
mod test {
    use std::{fs::File, path::PathBuf};

    use crate::Archive;

    fn data(name: &str) -> File {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/data")
            .join(name);

        File::open(path).unwrap()
    }

    #[test]
    fn read_header() {
        let mut a = Archive::open(data("rar5_stored.rar"));

        let main_header_block = a.next_block();
    }
}
