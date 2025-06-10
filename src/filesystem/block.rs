use core::ops::{Deref, DerefMut};

use crate::{Error, io::Writer};

/// Keeps a buffer of [`Block::LEN`] bytes.
#[derive(Eq, PartialEq)]
pub struct Block {
    inner: [u8; Self::LEN],
}

impl Block {
    /// The size of the block, most [`crate::BlockDevice`] like SD cards use blocks of 512 bytes.
    pub const LEN: usize = 512;

    /// Returns an empty block.
    pub const fn new() -> Self {
        Self { inner: [0u8; Self::LEN] }
    }

    /// Wraps a buffer.
    pub const fn wrap(inner: [u8; Self::LEN]) -> Self {
        Self { inner }
    }

    /// Copies [`Block::LEN`] bytes of a slice.
    ///
    /// # Errors
    ///
    /// Slice must be at least [`Block::LEN`] long.
    pub fn copy(from: &[u8]) -> Result<Block, Error> {
        if from.len() > Self::LEN {
            return Err(Error::BufferTooSmall { expected: Block::LEN, found: from.len() });
        }

        let mut block = Self::new();
        block[0..from.len()].copy_from_slice(from);
        Ok(block)
    }

    pub const fn bytes_mut(&mut self) -> &mut [u8] {
        &mut self.inner
    }

    pub fn writer(&mut self) -> Writer<'_> {
        Writer::new(&mut self.inner)
    }
}

impl Default for Block {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for Block {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Block {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl core::fmt::Debug for Block {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("[block]\n")?;
        for (i, byte) in self.inner.iter().enumerate() {
            f.write_fmt(format_args!(" {:02X}", byte))?;
            if (i + 1) % 32 == 0 {
                f.write_str("\n")?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn len() {
        assert_eq!(512, Block::LEN)
    }

    #[test]
    fn copy_too_big() {
        assert_eq!(
            Error::BufferTooSmall { expected: 512, found: 513 },
            Block::copy(&[8u8; Block::LEN + 1]).expect_err("should fail")
        );
    }

    #[test]
    fn copy_that_fits() {
        let actual = Block::copy(&[8u8; Block::LEN]).expect("should fit");
        assert_eq!([8u8; Block::LEN], actual.inner);
    }
}
