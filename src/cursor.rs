use thiserror::Error;

/// The maximum allowed size of a `vint`, in bytes.
const VINT_MAX_SIZE: u8 = 10;

pub type Result<T> = std::result::Result<T, ParseError>;

/// Holds onto a slice of bytes, and allows reading through it.
pub struct ByteCursor<'a> {
    bytes: &'a [u8],
    /// Points at the byte that will be read next.
    position: usize,
}

impl<'a> ByteCursor<'a> {
    pub fn from_bytes(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    /// Return the next [vint](https://www.rarlab.com/technote.htm#vint) in the byte array.
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
        let value = self.bytes.get(self.position).ok_or(ParseError::Truncated)?;

        self.position += 1;

        Ok(*value)
    }

    /// Read the next `n` bytes in the byte array. Use this function when we don't know the size of the
    /// data at compile time (e.g. file names, extra record bodies, etc.).
    pub fn read_bytes(&mut self, n: usize) -> Result<&[u8]> {
        let end = self.position.checked_add(n).ok_or(ParseError::Truncated)?;
        let bytes = self
            .bytes
            .get(self.position..end)
            .ok_or(ParseError::Truncated)?;

        self.position = end;

        Ok(bytes)
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

    /// Read the next `N` bytes in the byte array. Use this function when you know the size of data at
    /// compile time (e.g. integers).
    fn read_array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let bytes = self.bytes[self.position..]
            .split_first_chunk::<N>()
            .ok_or(ParseError::Truncated)?;

        self.position += N;

        Ok(*bytes.0)
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum ParseError {
    #[error("The `vint` was larger than the maximum allowed size of {VINT_MAX_SIZE} bytes.")]
    VintTooLong,
    #[error("The byte array ended unexpectedly.")]
    Truncated,
}

#[cfg(test)]
mod test {
    use crate::cursor::{ByteCursor, ParseError};
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
        let mut c = ByteCursor::from_bytes(bytes);
        assert_eq!(c.read_vint(), expected);
    }

    #[test]
    fn read_byte() {
        let bytes = [0x85];

        let mut c = ByteCursor::from_bytes(&bytes);

        assert_eq!(c.read_byte().unwrap(), bytes[0]);
        assert_eq!(c.read_byte(), Err(ParseError::Truncated));
    }

    #[test]
    fn read_bytes() {
        let bytes = [0x85, 0xF7, 0x81];

        let mut c = ByteCursor::from_bytes(&bytes);

        assert_eq!(c.read_bytes(3).unwrap(), bytes);
        assert_eq!(c.read_bytes(1), Err(ParseError::Truncated));
    }

    #[test]
    fn read_bytes_too_many() {
        let bytes = [0x85, 0xF7, 0x81];

        let mut c = ByteCursor::from_bytes(&bytes);

        assert_eq!(c.read_bytes(64), Err(ParseError::Truncated));
    }

    #[test]
    fn read_ints() {
        const BYTES: &[u8] = &[
            0x34, 0x12, // u16: 4660
            0x78, 0x56, 0x34, 0x12, // u32: 305419896
            0xEF, 0xCD, 0xAB, 0x90, 0x78, 0x56, 0x34, 0x12, // u64: 1311768467294899695
        ];

        let mut c = ByteCursor::from_bytes(BYTES);

        assert_eq!(c.read_u16().unwrap(), 4660);
        assert_eq!(c.read_u32().unwrap(), 305419896);
        assert_eq!(c.read_u64().unwrap(), 1311768467294899695);

        assert_eq!(c.read_byte(), Err(ParseError::Truncated));
    }
}
