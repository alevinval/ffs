use core::ops::{Deref, DerefMut};

use crate::io::{Reader, Writer};

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

    pub fn from_slice(slice: &[u8]) -> Self {
        let mut block = Self::new();
        block.inner[..slice.len()].copy_from_slice(slice);
        block
    }

    pub const fn bytes_mut(&mut self) -> &mut [u8] {
        &mut self.inner
    }

    pub const fn writer(&mut self) -> Writer<'_> {
        Writer::new(&mut self.inner)
    }

    pub const fn reader(&self) -> Reader<'_> {
        Reader::new(&self.inner)
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
        f.write_str("> Block:\n")?;
        for (i, byte) in self.inner.iter().enumerate() {
            f.write_fmt(format_args!(" {byte:02X}"))?;
            if (i + 1) % 32 == 0 {
                f.write_str("\n")?;
            }
        }
        Ok(())
    }
}
