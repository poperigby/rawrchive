use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bitflags::bitflags;
use nt_time::FileTime;

use crate::{cursor::ByteCursor, headers::HeaderFlags};

#[derive(Debug, Clone)]
pub struct MainHeader {
    extra_area_size: Option<u64>,
    archive_flags: ArchiveFlags,
    volume_number: Option<u64>,
    extra_area: Vec<ExtraRecord>,
}
impl MainHeader {
    pub fn new(mut cursor: ByteCursor, flags: HeaderFlags) -> Self {
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

        let extra_area = if flags.contains(HeaderFlags::EXTRA_AREA) {
            ExtraRecord::parse_all(&mut cursor)
        } else {
            Vec::new()
        };

        Self {
            extra_area_size,
            archive_flags,
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

#[derive(Debug, Clone)]
pub enum ExtraRecord {
    Locator(LocatorRecord),
    Metadata(MetadataRecord),
}

impl ExtraRecord {
    /// Parse out all extra area records using `cursor`, until we hit the end of the extra area.
    pub fn parse_all(cursor: &mut ByteCursor) -> Vec<ExtraRecord> {
        // TODO: We can't assume the amount of records in the extra area, so we'll just keep reading
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
                // TODO: Log a warning that we don't recognize this record
                _ => cursor.skip(size as usize),
            }
        }

        records
    }
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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
            let nanos = flags.contains(MetadataFlags::NANOSECONDS);

            let time = match (unix, nanos) {
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
