use std::{ffi::OsString, os::unix::ffi::OsStringExt};

use bitflags::bitflags;

use crate::cursor::ByteCursor;

/// See <https://www.rarlab.com/technote.htm#filehead:~:text=Unix%20owner%20record,-Size>.
#[derive(Debug, Clone, PartialEq)]
pub struct UnixOwnerRecord {
    user_name: Option<OsString>,
    user_id: Option<u64>,
    group_name: Option<OsString>,
    group_id: Option<u64>,
}

impl UnixOwnerRecord {
    pub fn parse(cursor: &mut ByteCursor) -> Self {
        let flags = Flags::from_bits_retain(cursor.read_vint().unwrap());

        let user_name = if flags.contains(Flags::USER_NAME) {
            let len = cursor.read_vint().unwrap();
            let bytes = cursor.read_bytes(len as usize).unwrap().to_vec();

            Some(OsString::from_vec(bytes))
        } else {
            None
        };

        let group_name = if flags.contains(Flags::GROUP_NAME) {
            let len = cursor.read_vint().unwrap();
            let bytes = cursor.read_bytes(len as usize).unwrap().to_vec();

            Some(OsString::from_vec(bytes))
        } else {
            None
        };

        let user_id = if flags.contains(Flags::USER_ID) {
            Some(cursor.read_vint().unwrap())
        } else {
            None
        };

        let group_id = if flags.contains(Flags::GROUP_ID) {
            Some(cursor.read_vint().unwrap())
        } else {
            None
        };

        Self {
            user_name,
            user_id,
            group_name,
            group_id,
        }
    }
}

bitflags! {
    /// See <https://www.rarlab.com/technote.htm#filehead:~:text=0x06-,Flags>.
    #[derive(Debug, Clone, PartialEq)]
    pub struct Flags: u64 {
        const USER_NAME = 0x0001;
        const GROUP_NAME = 0x0002;
        const USER_ID = 0x0004;
        const GROUP_ID = 0x0005;
    }
}
