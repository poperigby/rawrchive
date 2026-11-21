use std::path::PathBuf;

use bitflags::bitflags;

use crate::{cursor::ByteCursor, headers::HeaderFlags};

#[derive(Debug, Clone, PartialEq)]
pub struct FileHeader {
    extra_area_size: Option<u64>,
    data_size: Option<u64>,
    flags: FileFlags,
    unpacked_size: u64,
    // TODO: Make into a custom type
    attributes: u64,
    // TODO: Use SystemTime
    mtime: Option<u32>,
    crc32: Option<u32>,
    // TODO: Make into a custom type
    compression_info: CompressionInfo,
    host_os: HostOs,
    name: PathBuf,
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

        let attributes = cursor.read_vint().unwrap();

        let mtime = if flags.contains(FileFlags::UNIX_TIME) {
            Some(cursor.read_u32().unwrap())
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
            0x0000 => HostOs::Windows,
            0x0001 => HostOs::Unix,
            _ => panic!("Unknown host OS"),
        };

        let name_len = cursor.read_vint().unwrap();
        let name_bytes = cursor.read_bytes(name_len as usize).unwrap().to_vec();
        let name = String::from_utf8(name_bytes).unwrap();

        Self {
            extra_area_size,
            data_size,
            flags,
            unpacked_size,
            attributes,
            mtime,
            crc32,
            compression_info,
            host_os,
            name: PathBuf::from(name),
        }
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
pub enum HostOs {
    Windows,
    Unix,
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use crate::{
        Archive,
        headers::{
            Header,
            file::{AlgorithmVersion, CompressionInfo, FileFlags, FileHeader, HostOs},
        },
    };

    fn header(test_data: &str) -> Header {
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
            header("libarchive/v5/stored.rar"),
            Header::File(FileHeader {
                extra_area_size: Some(11),
                data_size: Some(29),
                flags: FileFlags::CRC32,
                unpacked_size: 29,
                // -rw-r--r--
                attributes: 0o100644,
                mtime: None,
                crc32: Some(0x95A043B4),
                compression_info: CompressionInfo {
                    algorithm_version: AlgorithmVersion::V5Plus,
                    solid: false,
                    method: 0,
                    dictionary_size: 131072
                },
                host_os: HostOs::Unix,
                name: PathBuf::from("helloworld.txt"),
            })
        );
    }
}
