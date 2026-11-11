#![forbid(unsafe_code)]

use std::io::Read;

pub struct Archive {}

impl Archive {
    pub fn open<R: Read>(source: R) {}
}

#[cfg(test)]
mod tests {}
