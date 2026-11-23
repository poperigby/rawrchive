use std::io::{self, BufReader, Read, Seek, Take};

use crate::headers::Header;

#[derive(Debug)]
pub struct Block<'a, R>
where
    R: Read + Seek,
{
    source: &'a mut BufReader<R>,
    pub(crate) header: Header,
}

impl<'a, R> Block<'a, R>
where
    R: Read + Seek,
{
    pub fn next(source: &'a mut BufReader<R>) -> Self {
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

    /// Returns a [`Data`] area that can be [`Read`] or [`Seek`]ed. The source is limited to the
    /// size of the data area, so there is no danger of reading beyond it.
    pub fn data(&'a mut self) -> Option<Data<'a, R>> {
        if let Header::File(header) = &self.header
            && let Some(data_size) = header.data_size()
        {
            Some(Data {
                source: self.source.take(data_size),
            })
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct Data<'a, R>
where
    R: Read + Seek,
{
    source: Take<&'a mut BufReader<R>>,
}

impl<'a, R> Read for Data<'a, R>
where
    R: Read + Seek,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.source.read(buf)
    }
}

impl<'a, R> Seek for Data<'a, R>
where
    R: Read + Seek,
{
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.source.seek(pos)
    }
}

#[cfg(test)]
mod test {
    use crate::Archive;

    #[test]
    fn read_header() {
        let mut a = Archive::open_test("libarchive/v5/stored.rar");

        a.next_block();
    }
}
