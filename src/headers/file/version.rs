use crate::cursor::ByteCursor;

/// See <https://www.rarlab.com/technote.htm#filehead:~:text=File%20version%20record,-This>.
#[derive(Debug, Clone, PartialEq)]
pub struct VersionRecord {
    version: u64,
}

impl VersionRecord {
    pub fn parse(cursor: &mut ByteCursor) -> Self {
        // No flags are currently defined, so this will just be 0.
        let _flags = cursor.read_vint().unwrap();

        let version = cursor.read_vint().unwrap();

        Self { version }
    }
}
