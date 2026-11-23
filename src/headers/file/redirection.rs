use bitflags::bitflags;

use crate::cursor::ByteCursor;

/// See <https://www.rarlab.com/technote.htm#filehead:~:text=File%20system%20redirection%20record,-Size>.
#[derive(Debug, Clone, PartialEq)]
pub struct RedirectionRecord {
    kind: Kind,
    is_dir: bool,
    name: String,
}

impl RedirectionRecord {
    pub fn parse(cursor: &mut ByteCursor) -> Self {
        let kind = match cursor.read_vint().unwrap() {
            0x0001 => Kind::UnixSymlink,
            0x0002 => Kind::WindowsSymlink,
            0x0003 => Kind::WindowsJunction,
            0x0004 => Kind::Hardlink,
            0x0005 => Kind::Copy,
            _ => panic!("Unknown redirection record kind"),
        };

        let flags = Flags::from_bits_retain(cursor.read_vint().unwrap());

        let is_dir = flags.contains(Flags::DIR);

        let name_len = cursor.read_vint().unwrap();
        let name_bytes = cursor.read_bytes(name_len as usize).unwrap().to_vec();
        let name = String::from_utf8(name_bytes).unwrap();

        Self { kind, is_dir, name }
    }
}

/// See <https://www.rarlab.com/technote.htm#filehead:~:text=Redirection%20type>
#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    UnixSymlink,
    WindowsSymlink,
    WindowsJunction,
    Hardlink,
    Copy,
}

bitflags! {
    /// See <https://www.rarlab.com/technote.htm#filehead:~:text=copy-,Flags>
    #[derive(Debug, Clone, PartialEq)]
    pub struct Flags: u64 {
        const DIR = 0x0001;
    }
}
