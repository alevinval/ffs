use crate::{
    BlockDevice, Error,
    filesystem::{
        Addr, Block, Deserializable, Free, Layout, Serializable, StaticReadFromDevice,
        WriteToDevice,
    },
};

#[derive(Debug)]
pub struct FreeBlockAllocator {
    inner: [Free; Self::LEN],
    dirty: [bool; Self::LEN],
    last_pos: usize,
}

impl Default for FreeBlockAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl FreeBlockAllocator {
    pub const LEN: usize = Layout::FREE.len();

    /// Returns a [`FreeBlockAllocator`] instance with all addresses marked as free.
    pub const fn new() -> Self {
        Self { inner: [const { Free::new() }; Self::LEN], dirty: [false; Self::LEN], last_pos: 0 }
    }

    /// Attempts to allocate enough blocks to fit `file_size` bytes.
    pub fn allocate_bytes(&mut self, file_size: usize, out: &mut [Addr]) -> Result<(), Error> {
        self.allocate_n(file_size / Block::LEN, out)
    }

    /// Attempts to allocate `n` blocks and stores the allocated indices in the provided `buffer`.
    ///
    /// # Arguments
    /// - `n`: The number of blocks to allocate.
    /// - `out`: A mutable slice used to store the resulting allocated indices. Must be at least `n` in length.
    ///
    /// # Returns
    /// - `Ok(())` if exactly `n` blocks were successfully allocated and stored in `buffer[0..n]`.
    /// - `Err(Error::BufferTooSmall)` if the buffer is too small to hold `n` indices.
    /// - `Err(Error::StorageFull)` if fewer than `n` blocks could be allocated. In this case,
    ///   allocated blocks will be automatically released.
    ///
    pub fn allocate_n(&mut self, n: usize, out: &mut [Addr]) -> Result<(), Error> {
        if out.len() < n {
            return Err(Error::BufferTooSmall { expected: n, found: out.len() });
        }

        let mut count = 0;
        while count < n {
            match self.allocate() {
                Ok(addr) => {
                    out[count] = addr;
                    count += 1;
                }
                Err(_) => {
                    for addr in out.iter().take(count) {
                        self.release(*addr);
                    }
                    return Err(Error::StorageFull);
                }
            }
        }
        Ok(())
    }

    /// Attempts to allocate a single block from the storage pool.
    ///
    /// # Returns
    /// - `Ok(Addr)` if a block was successfully allocated.
    /// - `Err(Error::StorageFull)` if no free blocks are available.
    ///
    /// # Notes
    /// - Uses a circular scan starting from `self.last_pos` for improved allocation locality.
    /// - Updates `self.last_pos` to the most recent allocation position to avoid always starting from 0.
    pub fn allocate(&mut self) -> Result<Addr, Error> {
        let len = self.inner.len();

        for i in 0..len {
            let pos = (self.last_pos + i) % len;
            if let Some(addr) = self.inner[pos].allocate() {
                self.dirty[pos] = true;
                self.last_pos = pos;
                return Ok(addr + pos_to_addr(pos));
            }
        }

        Err(Error::StorageFull)
    }

    /// Releases an allocated block back into the pool.
    ///
    /// # Arguments
    /// - `addr`: The address of the block to release.
    ///
    /// # Notes
    /// - Safe to call multiple times on the same address, though redundant calls may have no effect.
    /// - May adjust `self.last_pos` to improve future allocation locality.
    pub const fn release(&mut self, addr: Addr) {
        let pos = addr_to_pos(addr);
        self.inner[pos].release(addr_to_offset(addr));
        self.dirty[pos] = true;
        if pos < self.last_pos {
            self.last_pos = pos;
        }
    }

    pub fn count_free_addresses(&self) -> Addr {
        self.inner.iter().map(|f| f.count_free_addresses()).sum()
    }
}

impl<D> WriteToDevice<D> for FreeBlockAllocator
where
    D: BlockDevice,
{
    fn write_to_device(&self, out: &mut D) -> Result<(), Error> {
        let mut block = Block::new();
        for (pos, free) in self.inner.iter().enumerate().filter(|(pos, _)| self.dirty[*pos]) {
            free.serialize(&mut block.writer())?;
            out.write_block(Layout::FREE.nth(pos as u32), &block)?;
        }

        Ok(())
    }
}

impl<D> StaticReadFromDevice<D> for FreeBlockAllocator
where
    D: BlockDevice,
{
    type Item = Self;

    fn read_from_device(device: &mut D) -> Result<Self, Error> {
        let mut allocator = Self::new();
        for i in 0..Self::LEN {
            let mut block = Block::new();
            device.read_block(Layout::FREE.nth(i as u32), &mut block)?;
            allocator.inner[i] = Free::deserialize(&mut block.reader())?;
        }
        Ok(allocator)
    }
}

const fn addr_to_offset(addr: Addr) -> u32 {
    addr % Free::SLOTS as u32
}

const fn addr_to_pos(addr: Addr) -> usize {
    addr as usize / Free::SLOTS
}

const fn pos_to_addr(pos: usize) -> Addr {
    (pos * Free::SLOTS) as Addr
}

#[cfg(test)]
mod test {
    use crate::test_utils::MockDevice;

    use super::*;

    fn take_nth_blocks(sut: &mut FreeBlockAllocator, n: usize) -> Result<Addr, Error> {
        let mut last = Ok(0);
        for _ in 0..n {
            last = sut.allocate();
        }
        last
    }

    #[test]
    fn allocate() {
        let mut sut = FreeBlockAllocator::new();
        assert_eq!(8192, sut.count_free_addresses());
        assert_eq!(Ok(0), sut.allocate());
        assert_eq!(8191, sut.count_free_addresses());

        assert_eq!(Ok(8191), take_nth_blocks(&mut sut, 8191));
        assert_eq!(0, sut.count_free_addresses());
    }

    #[test]
    fn release() {
        let mut sut = FreeBlockAllocator::new();
        assert_eq!(Ok(8191), take_nth_blocks(&mut sut, 8192));
        assert_eq!(0, sut.count_free_addresses());

        sut.release(4000);
        sut.release(5000);
        sut.release(6000);
        assert_eq!(3, sut.count_free_addresses());

        assert_eq!(Ok(4000), sut.allocate());
        assert_eq!(Ok(5000), sut.allocate());
        assert_eq!(Ok(6000), sut.allocate());
    }

    #[test]
    fn allocate_n() {
        let mut sut = FreeBlockAllocator::new();
        assert_eq!(Ok(8191), take_nth_blocks(&mut sut, 8192));
        assert_eq!(0, sut.count_free_addresses());

        let mut addrs = [0; 10];
        assert_eq!(Err(Error::StorageFull), sut.allocate_n(8, &mut addrs));

        // Release sparse addresses
        sut.release(100);
        sut.release(200);
        sut.release(300);
        sut.release(1000);
        sut.release(2000);
        sut.release(3000);
        sut.release(7500);
        sut.release(1300);

        assert_eq!(Ok(()), sut.allocate_n(8, &mut addrs));
        assert_eq!([100, 200, 300, 1000, 1300, 2000, 3000, 7500], addrs[0..8]);

        // Now reproduce a rollback
        addrs[0..8].iter().for_each(|n| sut.release(*n));

        assert_eq!(8, sut.count_free_addresses());
        assert_eq!(Err(Error::StorageFull), sut.allocate_n(10, &mut addrs));
        assert_eq!(8, sut.count_free_addresses());
    }

    #[test]
    fn write_and_read_to_device() {
        let mut out = MockDevice::new();
        let mut sut = FreeBlockAllocator::new();
        assert_eq!(Ok(0), sut.allocate());
        assert_eq!(Ok(1), sut.allocate());
        assert_eq!(Ok(2), sut.allocate());
        assert_eq!(8189, sut.count_free_addresses());

        assert_eq!(Ok(()), sut.write_to_device(&mut out));

        let loaded = FreeBlockAllocator::read_from_device(&mut out)
            .expect("read from device should succeed");

        assert_eq!(sut.count_free_addresses(), loaded.count_free_addresses());
    }
}
