use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bitflags::bitflags;
use nt_time::FileTime;

use crate::cursor::ByteCursor;

/// See <https://www.rarlab.com/technote.htm#filehead:~:text=File%20time%20record,-This>.
#[derive(Debug, Clone, PartialEq)]
pub struct TimeRecord {
    pub(crate) mtime: Option<SystemTime>,
    pub(crate) ctime: Option<SystemTime>,
    pub(crate) atime: Option<SystemTime>,
}

impl TimeRecord {
    pub fn parse(cursor: &mut ByteCursor) -> Self {
        let flags = Flags::from_bits_retain(cursor.read_vint().unwrap());

        let mut mtime = if flags.contains(Flags::MTIME) {
            if flags.contains(Flags::UNIX_TIME) {
                Some(UNIX_EPOCH + Duration::from_secs(cursor.read_u32().unwrap() as u64))
            } else {
                Some(SystemTime::from(FileTime::new(cursor.read_u64().unwrap())))
            }
        } else {
            None
        };

        let mut ctime = if flags.contains(Flags::CTIME) {
            if flags.contains(Flags::UNIX_TIME) {
                Some(UNIX_EPOCH + Duration::from_secs(cursor.read_u32().unwrap() as u64))
            } else {
                Some(SystemTime::from(FileTime::new(cursor.read_u64().unwrap())))
            }
        } else {
            None
        };

        let mut atime = if flags.contains(Flags::ATIME) {
            if flags.contains(Flags::UNIX_TIME) {
                Some(UNIX_EPOCH + Duration::from_secs(cursor.read_u32().unwrap() as u64))
            } else {
                Some(SystemTime::from(FileTime::new(cursor.read_u64().unwrap())))
            }
        } else {
            None
        };

        if let Some(mtime) = &mut mtime
            && flags.contains(Flags::UNIX_TIME)
            && flags.contains(Flags::UNIX_NSEC)
        {
            *mtime += Duration::from_nanos(cursor.read_u32().unwrap() as u64);
        }

        if let Some(ctime) = &mut ctime
            && flags.contains(Flags::UNIX_TIME)
            && flags.contains(Flags::UNIX_NSEC)
        {
            *ctime += Duration::from_nanos(cursor.read_u32().unwrap() as u64);
        }

        if let Some(atime) = &mut atime
            && flags.contains(Flags::UNIX_TIME)
            && flags.contains(Flags::UNIX_NSEC)
        {
            *atime += Duration::from_nanos(cursor.read_u32().unwrap() as u64);
        }

        Self {
            mtime,
            ctime,
            atime,
        }
    }
}

bitflags! {
    /// See <https://www.rarlab.com/technote.htm#filehead:~:text=0x03-,Flags>.
    #[derive(Debug, Clone, PartialEq)]
    pub struct Flags: u64 {
        const UNIX_TIME = 0x0001;
        const MTIME = 0x0002;
        const CTIME = 0x0004;
        const ATIME = 0x0008;
        const UNIX_NSEC = 0x0010;
    }
}
