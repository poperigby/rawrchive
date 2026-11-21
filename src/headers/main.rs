use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bitflags::bitflags;
use nt_time::FileTime;
use tracing::warn;

use crate::{cursor::ByteCursor, headers::HeaderFlags};

#[derive(Debug, Clone, PartialEq)]
pub struct MainHeader {
    extra_area_size: Option<u64>,
    flags: ArchiveFlags,
    volume_number: Option<u64>,
    extra_area: Vec<ExtraRecord>,
}
impl MainHeader {
    pub fn parse(cursor: &mut ByteCursor, header_flags: HeaderFlags) -> Self {
        let extra_area_size = if header_flags.contains(HeaderFlags::EXTRA_AREA) {
            Some(cursor.read_vint().unwrap())
        } else {
            None
        };

        let flags = ArchiveFlags::from_bits_retain(cursor.read_vint().unwrap());

        let volume_number = if flags.contains(ArchiveFlags::MULTI_VOLUME) {
            Some(cursor.read_vint().unwrap())
        } else {
            None
        };

        let extra_area = if header_flags.contains(HeaderFlags::EXTRA_AREA) {
            ExtraRecord::parse_all(cursor)
        } else {
            Vec::new()
        };

        Self {
            extra_area_size,
            flags,
            volume_number,
            extra_area,
        }
    }
}

bitflags! {
    /// See `Archive flags` field in <https://www.rarlab.com/technote.htm#mainhead>.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ArchiveFlags: u64 {
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

#[derive(Debug, Clone, PartialEq)]
pub enum ExtraRecord {
    Locator(LocatorRecord),
    Metadata(MetadataRecord),
}

impl ExtraRecord {
    /// Parse out all extra area records using `cursor`, until we hit the end of the extra area.
    pub fn parse_all(cursor: &mut ByteCursor) -> Vec<ExtraRecord> {
        // We can't assume the amount of records in the extra area, so we'll just keep reading
        // until we run out of extra area bytes.

        let mut records = Vec::new();

        // The extra area is at the end of the header, so we can just loop until we run out of bytes
        // in the header's cursor.
        while cursor.remaining() != 0 {
            let size = cursor.read_vint().unwrap();
            let kind = cursor.read_vint().unwrap();
            match kind {
                1 => records.push(Self::Locator(LocatorRecord::parse(cursor))),
                2 => records.push(Self::Metadata(MetadataRecord::parse(cursor))),
                unknown => {
                    warn!(
                        "Unrecognized extra record flag in main header: {unknown}. Skipping over it."
                    );
                    cursor.skip(size as usize)
                }
            }
        }

        records
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LocatorRecord {
    quick_open_offset: Option<u64>,
    recovery_offset: Option<u64>,
}
impl LocatorRecord {
    fn parse(cursor: &mut ByteCursor) -> Self {
        let flags = LocatorFlags::from_bits_retain(cursor.read_vint().unwrap());

        let quick_open_offset = flags
            .contains(LocatorFlags::QUICK_OPEN)
            .then(|| cursor.read_vint().unwrap())
            .filter(|&offset| offset != 0);

        let recovery_offset = flags
            .contains(LocatorFlags::RECOVERY)
            .then(|| cursor.read_vint().unwrap())
            .filter(|&offset| offset != 0);

        Self {
            quick_open_offset,
            recovery_offset,
        }
    }
}

bitflags! {
    /// See `Flags` field in <https://www.rarlab.com/technote.htm#locrecord>.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct LocatorFlags: u64 {
        /// There is an offset provided for a quick open record
        const QUICK_OPEN = 0x0001;
        /// There is an offset provided for a recovery record
        const RECOVERY = 0x0002;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetadataRecord {
    name: Option<String>,
    time: Option<SystemTime>,
}

impl MetadataRecord {
    fn parse(cursor: &mut ByteCursor) -> Self {
        let flags = MetadataFlags::from_bits_retain(cursor.read_vint().unwrap());

        let name = if flags.contains(MetadataFlags::NAME) {
            let len = cursor.read_vint().unwrap();
            let s = cursor.read_bytes(len as usize).unwrap();

            Some(String::from_utf8(s.to_vec()).unwrap())
        } else {
            None
        };

        let time = if flags.contains(MetadataFlags::ORIGINAL_CREATION_TIME) {
            let unix = flags.contains(MetadataFlags::UNIX_TIME);
            let nanoseconds = flags.contains(MetadataFlags::NANOSECONDS);

            let time = match (unix, nanoseconds) {
                // Windows FILETIME
                (false, _) => SystemTime::from(FileTime::new(cursor.read_u64().unwrap())),
                // Unix time without nanoseconds
                (true, false) => {
                    UNIX_EPOCH + Duration::from_secs(cursor.read_u32().unwrap().into())
                }
                // Unix time with nanoseconds
                (true, true) => UNIX_EPOCH + Duration::from_nanos(cursor.read_u64().unwrap()),
            };

            Some(time)
        } else {
            None
        };

        Self { name, time }
    }
}

bitflags! {
    /// See `Flags` field in <https://www.rarlab.com/technote.htm#metarecord>.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct MetadataFlags: u64 {
        /// An archive name is present.
        const NAME = 0x0001;
        /// An original creation time is present.
        const ORIGINAL_CREATION_TIME = 0x0002;
        ///  Use Unix time if 1, Windows FILETIME if 0.
        const UNIX_TIME = 0x0004;
        /// Defines Unix time as nanoseconds since 1970-01-01 if 1 and as seconds since 1970-01-01 if 0.
        const NANOSECONDS = 0x0008;
    }
}

#[cfg(test)]
mod test {
    use std::time::UNIX_EPOCH;

    use nt_time::time::Duration;

    use crate::{
        Archive,
        headers::main::{ArchiveFlags, ExtraRecord, LocatorRecord, MainHeader, MetadataRecord},
    };

    fn header(test_data: &str) -> MainHeader {
        let a = Archive::open_test(test_data);
        a.main_header
    }

    #[test]
    fn stored() {
        assert_eq!(
            header("libarchive/v5/stored.rar"),
            MainHeader {
                extra_area_size: Some(6),
                flags: ArchiveFlags::empty(),
                volume_number: None,
                extra_area: vec![ExtraRecord::Locator(LocatorRecord {
                    quick_open_offset: None,
                    recovery_offset: None,
                })]
            }
        );
    }

    #[test]
    fn solid() {
        assert_eq!(
            header("libarchive/v5/solid.rar"),
            MainHeader {
                extra_area_size: Some(7),
                flags: ArchiveFlags::SOLID,
                volume_number: None,
                extra_area: vec![ExtraRecord::Locator(LocatorRecord {
                    quick_open_offset: None,
                    recovery_offset: None,
                })]
            }
        );
    }

    #[test]
    fn arm() {
        assert_eq!(
            header("libarchive/v5/arm.rar"),
            MainHeader {
                extra_area_size: Some(8),
                flags: ArchiveFlags::empty(),
                volume_number: None,
                extra_area: vec![ExtraRecord::Locator(LocatorRecord {
                    quick_open_offset: Some(40982),
                    recovery_offset: None,
                })]
            }
        );
    }

    #[test]
    fn extra_field_underflow() {
        assert_eq!(
            header("libarchive/v5/extra_field_underflow.rar"),
            MainHeader {
                extra_area_size: None,
                flags: ArchiveFlags::empty(),
                volume_number: None,
                extra_area: Vec::new(),
            }
        );
    }

    #[test]
    fn main_block_extra_bytes() {
        assert_eq!(
            // Contains 8 bytes of 0x00 padding that will just be ignored.
            header("libarchive/v5/main_block_extra_bytes.rar"),
            MainHeader {
                extra_area_size: Some(6),
                flags: ArchiveFlags::empty(),
                volume_number: None,
                extra_area: vec![ExtraRecord::Locator(LocatorRecord {
                    quick_open_offset: None,
                    recovery_offset: None
                })]
            }
        );
    }

    #[test]
    fn metadata_filetime() {
        assert_eq!(
            header("rawrchive/v5/metadata_filetime.rar"),
            MainHeader {
                extra_area_size: Some(24),
                flags: ArchiveFlags::empty(),
                volume_number: None,
                extra_area: vec![ExtraRecord::Metadata(MetadataRecord {
                    name: Some("metadata.rar".into()),
                    time: Some(UNIX_EPOCH + Duration::new(1_795_120_205, 921_933_900))
                })]
            }
        );
    }

    #[test]
    fn metadata_unix() {
        assert_eq!(
            header("rawrchive/v5/metadata_unix.rar"),
            MainHeader {
                extra_area_size: Some(20),
                flags: ArchiveFlags::empty(),
                volume_number: None,
                extra_area: vec![ExtraRecord::Metadata(MetadataRecord {
                    name: Some("metadata.rar".into()),
                    time: Some(UNIX_EPOCH + Duration::new(1_795_120_205, 0))
                })]
            }
        );
    }

    #[test]
    fn metadata_unix_ns() {
        assert_eq!(
            header("rawrchive/v5/metadata_unix_ns.rar"),
            MainHeader {
                extra_area_size: Some(24),
                flags: ArchiveFlags::empty(),
                volume_number: None,
                extra_area: vec![ExtraRecord::Metadata(MetadataRecord {
                    name: Some("metadata.rar".into()),
                    time: Some(UNIX_EPOCH + Duration::new(1_795_120_205, 921_933_900))
                })]
            }
        );
    }
}
