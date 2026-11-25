use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bitflags::bitflags;
use tracing::warn;

use crate::{
    cursor::ByteCursor,
    headers::{
        HeaderFlags,
        file::{
            encryption::EncryptionRecord, hash::HashRecord, redirection::RedirectionRecord,
            time::TimeRecord, unix_owner::UnixOwnerRecord, version::VersionRecord,
        },
    },
};

mod encryption;
mod hash;
mod redirection;
mod time;
mod unix_owner;
mod version;

/// See <https://www.rarlab.com/technote.htm#filehead:~:text=1%2E-,File%20header%20and%20service%20header>.
#[derive(Debug, Clone, PartialEq)]
pub struct FileHeader {
    extra_area_size: Option<u64>,
    data_size: Option<u64>,
    flags: FileFlags,
    unpacked_size: u64,
    attributes: Attributes,
    mtime: Option<SystemTime>,
    crc32: Option<u32>,
    compression_info: CompressionInfo,
    // TODO: Provide a method that validates and returns this as a PathBuf
    // It will also have to map this bullshit back to the original bytes:
    // https://www.rarlab.com/technote.htm#filehead:~:text=If,-Unix
    name: String,
    extra_area: Vec<ExtraRecord>,
}

impl FileHeader {
    pub fn parse(cursor: &mut ByteCursor, header_flags: HeaderFlags) -> Self {
        let extra_area_size = if header_flags.contains(HeaderFlags::EXTRA_AREA) {
            Some(cursor.read_vint().unwrap())
        } else {
            None
        };

        let data_size = if header_flags.contains(HeaderFlags::DATA_AREA) {
            Some(cursor.read_vint().unwrap())
        } else {
            None
        };

        let flags = FileFlags::from_bits_retain(cursor.read_vint().unwrap());

        let unpacked_size = cursor.read_vint().unwrap();

        let attribute_bytes = cursor.read_vint().unwrap();

        let mtime = if flags.contains(FileFlags::UNIX_TIME) {
            // For some reason this field is a u32, even though Unix defines mtime `tv_sec` as a `long`.
            // https://man7.org/linux/man-pages/man7/inode.7.html#:~:text=Last%20modification%20timestamp%20%28mtime%29
            // https://man7.org/linux/man-pages/man3/timespec.3type.html
            Some(UNIX_EPOCH + Duration::from_secs(cursor.read_u32().unwrap() as u64))
        } else {
            None
        };

        let crc32 = if flags.contains(FileFlags::CRC32) {
            Some(cursor.read_u32().unwrap())
        } else {
            None
        };

        let compression_info = CompressionInfo::parse(cursor.read_vint().unwrap());

        let host_os = match cursor.read_vint().unwrap() {
            0x0001 => Os::Unix,
            0x0000 => Os::Windows,
            _ => panic!("Unknown host OS"),
        };

        let name_len = cursor.read_vint().unwrap();
        let name_bytes = cursor.read_bytes(name_len as usize).unwrap().to_vec();

        // "Forward slash character is used as the path separator both for Unix and Windows names."
        // https://www.rarlab.com/technote.htm#filehead:~:text=Forward
        let name = String::from_utf8(name_bytes).unwrap();
        // "Backslashes are treated as a part of name for Unix names and as invalid character for Windows file names."
        // https://www.rarlab.com/technote.htm#filehead:~:text=Backslashes
        //
        // Valid writers shouldn't allow an archive with a name containing `\` to be made, but we
        // still need to guard against it in the case of a corrupted archive or broken writer.
        if host_os == Os::Windows && name.contains("\\") {
            panic!("Windows file names shouldn't contain a backslash");
        }

        let extra_area = if header_flags.contains(HeaderFlags::EXTRA_AREA) {
            ExtraRecord::parse_all(cursor)
        } else {
            Vec::new()
        };

        Self {
            extra_area_size,
            data_size,
            flags,
            unpacked_size,
            attributes: match host_os {
                Os::Unix => Attributes::Unix(attribute_bytes),
                Os::Windows => Attributes::Windows(attribute_bytes),
            },
            mtime,
            crc32,
            compression_info,
            name,
            extra_area,
        }
    }

    pub fn data_size(&self) -> Option<u64> {
        self.data_size
    }
}

bitflags! {
    /// See `File flags` field at <https://www.rarlab.com/technote.htm#filehead:~:text=File%20flags>.
    #[derive(Debug, Clone, PartialEq)]
    pub struct FileFlags: u64 {
        /// This entry is a directory.
        const DIRECTORY = 0x0001;
        /// This header has a time field in the Unix format.
        const UNIX_TIME = 0x0002;
        /// This header has a CRC32 field for the data area.
        const CRC32 = 0x0004;
        /// The unpacked size of this file is unknown. The `unpacked_size` field is still present,
        /// but it must be ignored, and extraction must continue until the end of the compression
        /// stream is reached.
        const UNKNOWN_UNPACKED_SIZE = 0x0008;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Attributes {
    /// Stores attributes in `st_mode` format: <https://man7.org/linux/man-pages/man7/inode.7.html#:~:text=The%20file%20type%20and%20mode>.
    Unix(u64),
    Windows(u64),
}

/// See `Compression information` field at <https://www.rarlab.com/technote.htm#filehead:~:text=Compression%20information>.
#[derive(Debug, Clone, PartialEq)]
pub struct CompressionInfo {
    algorithm_version: AlgorithmVersion,
    solid: bool,
    method: u8,
    dictionary_size: u64,
}

impl CompressionInfo {
    pub fn parse(value: u64) -> Self {
        // https://www.rarlab.com/technote.htm#filehead:~:text=0x003f
        let mut algorithm_version = match value & 0x003F {
            0 => AlgorithmVersion::V5Plus,
            1 => AlgorithmVersion::V7Plus,
            _ => panic!("Unrecognized algorithm version"),
        };

        // https://www.rarlab.com/technote.htm#filehead:~:text=bit%20%28-,0x0040
        let solid = value & 0x0040 != 0;

        // https://www.rarlab.com/technote.htm#filehead:~:text=0x0380
        let method = ((value & 0x0380) >> 7) as u8;

        // https://www.rarlab.com/technote.htm#filehead:~:text=0x7c00
        let dictionary_exponent = (value & 0x7C00) >> 10;
        let mut dictionary_size = (128u64 * 1024u64) << dictionary_exponent;

        // v7+ algorithm needs a bigger dictionary
        if algorithm_version == AlgorithmVersion::V7Plus {
            // https://www.rarlab.com/technote.htm#filehead:~:text=0xf8000
            let dictionary_fraction = (value & 0xf8000) >> 15;

            dictionary_size += (dictionary_size * dictionary_fraction) / 32;
        }

        // https://www.rarlab.com/technote.htm#filehead:~:text=0x100000
        if (value & 0x100000) >> 20 == 1 {
            // We still need the v7+ dictionary size for solid archives, because all the files share
            // a dictionary, but we want the v5+ decoding algorithm for this file.
            algorithm_version = AlgorithmVersion::V5Plus;
        }

        Self {
            algorithm_version,
            solid,
            method,
            dictionary_size,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AlgorithmVersion {
    V5Plus,
    V7Plus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Os {
    Unix,
    Windows,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExtraRecord {
    Encryption(EncryptionRecord),
    Hash(HashRecord),
    Time(TimeRecord),
    Version(VersionRecord),
    Redirection(RedirectionRecord),
    UnixOwner(UnixOwnerRecord),
}

impl ExtraRecord {
    pub fn parse_all(cursor: &mut ByteCursor) -> Vec<ExtraRecord> {
        let mut records = Vec::new();

        while cursor.remaining() != 0 {
            // TODO: Hmm maybe we should be doing something with this size?
            let size = cursor.read_vint().unwrap();
            let kind = cursor.read_vint().unwrap();
            records.push(match kind {
                0x01 => ExtraRecord::Encryption(EncryptionRecord::parse(cursor)),
                0x02 => ExtraRecord::Hash(HashRecord::parse(cursor)),
                0x03 => ExtraRecord::Time(TimeRecord::parse(cursor)),
                0x04 => ExtraRecord::Version(VersionRecord::parse(cursor)),
                0x05 => ExtraRecord::Redirection(RedirectionRecord::parse(cursor)),
                0x06 => ExtraRecord::UnixOwner(UnixOwnerRecord::parse(cursor)),
                unknown => {
                    warn!(
                        "Unrecognized extra record flag in main header: {unknown}. Skipping over it."
                    );
                    cursor.skip(size as usize);

                    break;
                }
            });
        }

        records
    }
}

#[cfg(test)]
mod test {
    use std::time::{Duration, UNIX_EPOCH};

    use pretty_assertions::assert_eq;

    use crate::{
        Archive,
        headers::{
            Header,
            file::{
                AlgorithmVersion, Attributes, CompressionInfo, ExtraRecord, FileFlags, FileHeader,
                time::TimeRecord,
            },
        },
    };

    fn first_header(test_data: &str) -> Header {
        let mut a = Archive::open_test(test_data);

        let mut block = a.next_block();
        while !matches!(block.header, Header::File(_)) {
            block = a.next_block();
        }

        block.header
    }

    #[test]
    fn stored() {
        assert_eq!(
            first_header("libarchive/v5/stored.rar"),
            Header::File(FileHeader {
                extra_area_size: Some(11),
                data_size: Some(29),
                flags: FileFlags::CRC32,
                unpacked_size: 29,
                // -rw-r--r--
                attributes: Attributes::Unix(0o100644),
                mtime: None,
                crc32: Some(0x95A043B4),
                compression_info: CompressionInfo {
                    algorithm_version: AlgorithmVersion::V5Plus,
                    solid: false,
                    method: 0,
                    dictionary_size: 131072
                },
                name: "helloworld.txt".to_string(),
                extra_area: vec![ExtraRecord::Time(TimeRecord {
                    mtime: Some(UNIX_EPOCH + Duration::new(1_537_937_022, 437_184_854)),
                    ctime: None,
                    atime: None,
                })],
            })
        );
    }
}
