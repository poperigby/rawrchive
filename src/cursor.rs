use thiserror::Error;

/// The maximum size of a `vint`, in bytes.
const VINT_MAX_SIZE: u8 = 10;

pub type Result<T> = std::result::Result<T, HeaderError>;

/// Holds onto a header slice and allows reading through
pub struct HeaderCursor<'a> {
    header: &'a [u8],
    position: usize,
}

impl<'a> HeaderCursor<'a> {
    pub fn new(header: &'a [u8]) -> Self {
        Self {
            header,
            position: 0,
        }
    }

    pub fn read_byte(&mut self) -> Result<u8> {
        let value = self
            .header
            .get(self.position)
            .ok_or(HeaderError::Truncated)?;

        self.position += 1;

        Ok(*value)
    }

    /// Return the next [vint](https://www.rarlab.com/technote.htm#vint) in the header.
    pub fn read_vint(&mut self) -> Result<u64> {
        let mut result: u64 = 0;
        let mut shift = 0;

        for _ in 0..VINT_MAX_SIZE {
            let byte = self.read_byte()?;

            // The lower 7 bits contain the integer data
            let data = byte & 0x7F;

            result |= (data as u64) << shift;

            // The highest bit contains the continue flag
            if byte & 0x80 != 0 {
                shift += 7;
                continue;
            } else {
                return Ok(result);
            }
        }

        Err(HeaderError::VintTooLong)
    }
}

#[derive(Debug, Error)]
pub enum HeaderError {
    #[error("The vint was larger than the maximum allowed size of {VINT_MAX_SIZE} bytes.")]
    VintTooLong,
    #[error("The header unexpectedly ended")]
    Truncated,
}

#[cfg(test)]
mod test {
    use crate::cursor::HeaderCursor;

    #[test]
    fn read_vint() {
        let cases: &[(&[u8], u64)] = &[
            (&[0x05], 5),
            (&[0x7F], 127),
            (&[0x80, 0x01], 128),
            (&[0x85, 0x02], 261),
        ];
        for (bytes, expected) in cases {
            let mut c = HeaderCursor::new(bytes);
            assert_eq!(c.read_vint().unwrap(), *expected);
        }
    }
}
