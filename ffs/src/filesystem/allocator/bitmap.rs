use crate::{
    Error,
    filesystem::{Addr, Block, Deserializable, SerdeLen, Serializable},
    io::{Read, Write},
};

/// Tracks the free status of an address space, represented as a bitmap.
#[derive(PartialEq, Eq, Debug)]
pub struct Bitmap {
    /// Block of bytes that is actually persisted.
    block: Block,

    /// The last free position that has been seen, computed on-demand.
    last_free_pos: usize,
}

impl Default for Bitmap {
    fn default() -> Self {
        Self::new()
    }
}

impl Bitmap {
    /// The number of addresses that can be tracked by a single [`Bitmap`] instance.
    ///
    /// Each [`Block`] contains [`Block::LEN`] bytes, each bit in the byte represents
    /// the free status of an address within the address space.
    ///
    /// Thus, a single [`Bitmap`] can track 4096 addresses.
    pub const SLOTS: usize = 8 * Block::LEN;

    /// Returns a [`Bitmap`] instance with all addresses marked as free.
    pub const fn new() -> Self {
        Self { block: Block::new(), last_free_pos: 0 }
    }

    /// Counts number of free addresses.
    pub fn count_free_addresses(&self) -> usize {
        let mut n = 0;
        for octet in self.block.iter() {
            n += u8::count_zeros(*octet);
        }
        n as usize
    }

    /// Takes the first available address.
    ///
    /// This method relies on [`Self::last_free_pos`] heuristic to skip regions of the block
    /// without free addresses available.
    ///
    /// Returns `Some(Addr)` if successful, or `None` if no free blocks remain.
    pub fn take(&mut self) -> Option<Addr> {
        for (pos, byte) in self.block.iter_mut().skip(self.last_free_pos).enumerate() {
            let taken_bits = u8::trailing_ones(*byte);
            if taken_bits < u8::BITS {
                *byte |= 1 << taken_bits;
                self.last_free_pos += pos;
                return Some((8 * self.last_free_pos as Addr) + taken_bits as Addr);
            }
        }
        None
    }

    /// Releases an address and makes it available to be taken again.
    ///
    /// Updates the [`Self::last_free_pos`] heuristic.
    pub fn release(&mut self, addr: Addr) {
        let shift = addr % 8;
        let pos = (addr / 8) as usize;
        self.block[pos] &= !(1 << shift);
        if pos < self.last_free_pos {
            self.last_free_pos = pos;
        }
    }
}

impl SerdeLen for Bitmap {
    const SERDE_LEN: usize = Block::LEN;
}

impl Serializable for Bitmap {
    /// Serializes the [`Bitmap`] instance into the provided byte slice.
    ///
    /// This copies the internal block state [`Self::inner`] into the first `[Block::LEN]` bytes of `out`.
    ///
    /// # Errors
    ///
    /// Returns an error if `out` is too small to hold the serialized data.
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<usize, Error> {
        let n = writer.write(&self.block)?;
        Ok(n)
    }
}

impl Deserializable<Self> for Bitmap {
    /// Deserializes a [`Bitmap`] instance from the given byte slice.
    ///
    /// This method copies the first [`Block::LEN`] bytes of `buf` into `[Self::inner]`
    ///
    /// # Errors
    ///
    /// Returns an error if `buf` is too small to contain a full [`Block`].
    fn deserialize<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let mut free = Self::new();
        reader.read(&mut free.block)?;
        Ok(free)
    }
}

#[cfg(test)]
mod tests {

    use crate::test_serde_symmetry;

    use super::*;

    fn take_nth_blocks(sut: &mut Bitmap, n: usize) -> Option<Addr> {
        let mut last = None;
        for _ in 0..n {
            last = sut.take();
        }
        last
    }

    fn get_full_bitmap() -> Bitmap {
        let mut bitmap = Bitmap::new();
        take_nth_blocks(&mut bitmap, 2048);
        bitmap.last_free_pos = 0;
        bitmap
    }

    test_serde_symmetry!(Bitmap, get_full_bitmap());

    #[test]
    fn test_count_free_addresses() {
        let sut = Bitmap::new();
        assert_eq!(4096, sut.count_free_addresses());
    }

    #[test]
    fn test_take() {
        let mut sut = Bitmap::new();
        assert_eq!(Some(0), sut.take());
        assert_eq!(Some(1), sut.take());
        assert_eq!(Some(2), sut.take());
        assert_eq!(Some(4095), take_nth_blocks(&mut sut, 4093));
        assert_eq!(0, sut.count_free_addresses());
    }

    #[test]
    fn test_take_then_release() {
        let mut sut = Bitmap::new();
        assert_eq!(Some(4095), take_nth_blocks(&mut sut, 4096));
        assert_eq!(0, sut.count_free_addresses());

        sut.release(512);
        sut.release(600);
        sut.release(700);

        assert_eq!(3, sut.count_free_addresses());
        assert_eq!(Some(512), sut.take());

        assert_eq!(2, sut.count_free_addresses());
        assert_eq!(Some(600), sut.take());
    }
}
