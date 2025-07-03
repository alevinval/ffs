use crate::{
    Error,
    filesystem::{Addr, Block, Deserializable, SerdeLen, Serializable},
    io::{Read, Write},
};

/// Tracks the free status of an address space.
#[derive(PartialEq, Eq, Debug)]
pub struct Free {
    inner: Block,
    last_free: usize,
}

impl Default for Free {
    fn default() -> Self {
        Self::new()
    }
}

impl Free {
    /// The number of addresses that can be tracked by a single [`Free`] instance.
    ///
    /// Each [`Block`] contains `Block::LEN` bytes, each bit in the byte represents
    /// the free status of an address.
    pub const SLOTS: usize = 8 * Block::LEN;

    /// Returns a [`Free`] instance with all addresses marked as free.
    pub const fn new() -> Self {
        Self { inner: Block::new(), last_free: 0 }
    }

    /// Counts number of free addresses.
    pub fn count_free_addresses(&self) -> usize {
        let mut n = 0;
        for octet in self.inner.iter() {
            n += u8::count_zeros(*octet) as usize;
        }
        n
    }

    /// Attempts to allocate the first available address.
    ///
    /// This method uses the [`Self::last_free`] heuristic to skip over regions of the block
    /// where no free addresses were previously found, improving performance.
    ///
    /// Returns `Some(Addr)` if successful, or `None` if no free blocks remain.
    pub fn allocate(&mut self) -> Option<Addr> {
        for (pos, byte) in self.inner.iter_mut().skip(self.last_free).enumerate() {
            let taken_bits = u8::trailing_ones(*byte);
            if taken_bits < u8::BITS {
                *byte |= 1 << taken_bits;
                self.last_free += pos;
                return Some((8 * self.last_free as Addr) + taken_bits as Addr);
            }
        }
        None
    }

    /// Marks the address at the given `[Addr]` as free again.
    ///
    /// This restores a previously allocated address by flipping its bit
    /// in the underlying [`Block`] from `1` (used) to `0` (free).
    ///
    /// Updates the [`Self::last_free`] heuristic speed up future allocations.
    pub const fn release(&mut self, addr: Addr) {
        let shift = addr % 8;
        let pos = (addr / 8) as usize;
        self.inner.bytes_mut()[pos] &= !(1 << shift);
        if pos < self.last_free {
            self.last_free = pos;
        }
    }
}

impl SerdeLen for Free {
    const SERDE_LEN: usize = Block::LEN;
}

impl Serializable for Free {
    /// Serializes the [`Free`] instance into the provided byte slice.
    ///
    /// This copies the internal block state [`Self::inner`] into the first `[Block::LEN]` bytes of `out`.
    ///
    /// # Errors
    ///
    /// Returns an error if `out` is too small to hold the serialized data.
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<usize, Error> {
        let n = writer.write(&self.inner)?;
        Ok(n)
    }
}

impl Deserializable<Self> for Free {
    /// Deserializes a [`Free`] instance from the given byte slice.
    ///
    /// This method copies the first [`Block::LEN`] bytes of `buf` into `[Self::inner]`
    ///
    /// # Errors
    ///
    /// Returns an error if `buf` is too small to contain a full [`Block`].
    fn deserialize<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let mut free = Self::new();
        reader.read(&mut free.inner)?;
        Ok(free)
    }
}

#[cfg(test)]
mod test {

    use super::*;

    fn take_nth_blocks(sut: &mut Free, n: usize) -> Option<Addr> {
        let mut last = None;
        for _ in 0..n {
            last = sut.allocate();
        }
        last
    }

    #[test]
    fn count_free_addresses() {
        let sut = Free::new();
        assert_eq!(4096, sut.count_free_addresses())
    }

    #[test]
    fn take() {
        let mut sut = Free::new();
        assert_eq!(Some(0), sut.allocate());
        assert_eq!(Some(1), sut.allocate());
        assert_eq!(Some(2), sut.allocate());
        assert_eq!(Some(4095), take_nth_blocks(&mut sut, 4093));
        assert_eq!(0, sut.count_free_addresses());
    }

    #[test]
    fn release() {
        let mut sut = Free::new();
        assert_eq!(Some(4095), take_nth_blocks(&mut sut, 4096));
        assert_eq!(0, sut.count_free_addresses());

        sut.release(512);
        sut.release(600);
        sut.release(700);

        assert_eq!(3, sut.count_free_addresses());
        assert_eq!(Some(512), sut.allocate());

        assert_eq!(2, sut.count_free_addresses());
        assert_eq!(Some(600), sut.allocate());
    }

    #[test]
    fn serde_symmetry() {
        let mut expected = Free::new();
        take_nth_blocks(&mut expected, 2048);

        let mut block = Block::new();
        assert_eq!(Ok(Free::SERDE_LEN), expected.serialize(&mut block.writer()));
        let actual = Free::deserialize(&mut block.reader()).unwrap();

        assert_eq!(expected.inner, actual.inner);
    }
}
