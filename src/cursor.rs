use thiserror::Error;

/// The maximum size of a `vint`, in bytes.
const VINT_MAX_SIZE: u8 = 10;

pub type Result<T> = std::result::Result<T, ParseError>;

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

        Err(ParseError::VintTooLong)
    }

    pub fn read_byte(&mut self) -> Result<u8> {
        let value = self
            .header
            .get(self.position)
            .ok_or(ParseError::Truncated)?;

        self.position += 1;

        Ok(*value)
    }

    pub fn read_u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.read_array::<2>()?))
    }

    pub fn read_u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.read_array::<4>()?))
    }

    pub fn read_u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.read_array::<8>()?))
    }

    /// Read the next `N` bytes in the header. Use this function when you know the size of data at
    /// compile time (e.g. integers).
    fn read_array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let bytes = self.header[self.position..]
            .split_first_chunk::<N>()
            .ok_or(ParseError::Truncated)?;

        Ok(*bytes.0)
    }

    /// Read the next `n` bytes in the header. Use this function when we don't know the size of the
    /// data at compile time (e.g. file names, extra record bodies, etc.).
    fn read_bytes(&mut self, n: usize) -> Result<&[u8]> {
        let end = self.position.checked_add(n).ok_or(ParseError::Truncated)?;
        let bytes = self
            .header
            .get(self.position..end)
            .ok_or(ParseError::Truncated)?;

        self.position = end;

        Ok(bytes)
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum ParseError {
    #[error("The vint was larger than the maximum allowed size of {VINT_MAX_SIZE} bytes.")]
    VintTooLong,
    #[error("The header unexpectedly ended")]
    Truncated,
}

#[cfg(test)]
mod test {
    use crate::cursor::{HeaderCursor, ParseError};
    use rstest::rstest;

    #[rstest]
    #[case::one_byte(&[0x05], Ok(5))]
    #[case::max_one_byte(&[0x7F], Ok(127))]
    #[case::two_bytes_zero_first(&[0x80, 0x01], Ok(128))]
    #[case::two_bytes(&[0x85, 0x02], Ok(261))]
    #[case::max_valid(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01], Ok(u64::MAX))]
    #[case::too_long(&[0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x01], Err(ParseError::VintTooLong))]
    #[case::ten_continues(&[0x80; 10], Err(ParseError::VintTooLong))]
    #[case::truncated(&[0x80], Err(ParseError::Truncated))]
    #[case::empty(&[], Err(ParseError::Truncated))]
    fn read_vint(#[case] bytes: &[u8], #[case] expected: Result<u64, ParseError>) {
        let mut c = HeaderCursor::new(bytes);
        assert_eq!(c.read_vint(), expected);
    }
}
