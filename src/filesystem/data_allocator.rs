use crate::{
    BlockDevice, Error,
    filesystem::{Addr, Block, Deserializable, Free, Node, Serializable, layout::Layout},
};

#[derive(Debug)]
pub struct DataAllocator {
    layout: Layout,
    last_accessed: Addr,
}

impl DataAllocator {
    pub const fn new(layout: Layout) -> Self {
        Self { last_accessed: 0, layout }
    }

    /// Attempts to allocate enough blocks to fit `file_size` bytes and returns a [`Node`] instance
    /// with all the allocated addresses.
    pub fn allocate_node_data<D: BlockDevice>(
        &mut self,
        device: &mut D,
        file_size: usize,
    ) -> Result<Node, Error> {
        let mut block_addrs = [0; Node::BLOCKS_PER_NODE];
        self.allocate_bytes(device, file_size, &mut block_addrs)?;
        Ok(Node::new(file_size as u16, block_addrs))
    }

    /// Releases all blocks associated with the given [`Node`].
    pub fn release_node_data<D: BlockDevice>(
        &mut self,
        device: &mut D,
        node: &Node,
    ) -> Result<(), Error> {
        for addr in node.block_addrs() {
            self.release(device, *addr)?;
        }
        Ok(())
    }

    pub fn count_free_addresses<D: BlockDevice>(&self, device: &mut D) -> Result<usize, Error> {
        let mut block = Block::new();
        let mut total = 0;

        for sector in self.layout.iter_sectors() {
            device.read_block(sector, &mut block)?;
            let free = Free::deserialize(&mut block.reader())?;
            total += free.count_free_addresses();
        }
        Ok(total)
    }

    /// Attempts to allocate enough blocks to fit `file_size` bytes.
    fn allocate_bytes<D: BlockDevice>(
        &mut self,
        device: &mut D,
        file_size: usize,
        out: &mut [Addr],
    ) -> Result<(), Error> {
        self.allocate_n(device, file_size.div_ceil(Block::LEN), out)
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
    fn allocate_n<D: BlockDevice>(
        &mut self,
        device: &mut D,
        n: usize,
        out: &mut [Addr],
    ) -> Result<(), Error> {
        if out.len() < n {
            return Err(Error::BufferTooSmall { expected: n, found: out.len() });
        }

        let mut count = 0;
        while count < n {
            match self.allocate(device) {
                Ok(addr) => {
                    out[count] = addr;
                    count += 1;
                }
                Err(_) => {
                    for addr in out.iter().take(count) {
                        self.release(device, *addr)?;
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
    /// - Uses a circular scan starting from `self.last_accessed` for improved allocation locality.
    /// - Updates `self.last_accessed` to the most recent allocation position to avoid always starting from 0.
    pub fn allocate<D: BlockDevice>(&mut self, device: &mut D) -> Result<Addr, Error> {
        let mut block = Block::new();

        for (addr, sector) in self.layout.circular_iter(self.last_accessed) {
            device.read_block(sector, &mut block)?;
            let mut free = Free::deserialize(&mut block.reader())?;

            if let Some(allocation) = free.allocate() {
                free.serialize(&mut block.writer())?;
                device.write_block(sector, &block)?;
                self.last_accessed = addr;
                return Ok(to_data_sector(addr, allocation));
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
    /// - May adjust `self.last_accessed` to improve future allocation locality.
    pub fn release<D: BlockDevice>(
        &mut self,
        device: &mut D,
        data_sector: Addr,
    ) -> Result<(), Error> {
        let logical_addr = to_free_logical(data_sector) as Addr;
        let free_sector = self.layout.nth(logical_addr);
        let offset = to_allocated_offset(data_sector);

        let mut block = Block::new();
        device.read_block(free_sector, &mut block)?;
        let mut free = Free::deserialize(&mut block.reader())?;

        free.release(offset);
        free.serialize(&mut block.writer())?;
        device.write_block(free_sector, &block)?;
        if logical_addr < self.last_accessed {
            self.last_accessed = logical_addr;
        }
        Ok(())
    }
}

const fn to_allocated_offset(addr: Addr) -> Addr {
    addr % Free::SLOTS as Addr
}

const fn to_free_logical(addr: Addr) -> usize {
    addr as usize / Free::SLOTS
}

const fn to_data_sector(free_sector: Addr, allocation: Addr) -> Addr {
    free_sector * Free::SLOTS as Addr + allocation
}

#[cfg(test)]
mod test {
    use crate::disk::MemoryDisk;

    use super::*;

    const TEST_LAYOUT: Layout = Layout::new(0, 2);

    fn get_sut() -> (MemoryDisk, DataAllocator) {
        let device = MemoryDisk::fit(TEST_LAYOUT.sector_count());
        let sut = DataAllocator::new(TEST_LAYOUT);
        (device, sut)
    }

    fn take_nth_blocks<D: BlockDevice>(
        sut: &mut DataAllocator,
        device: &mut D,
        n: usize,
    ) -> Result<Addr, Error> {
        let mut last = Ok(0);
        for _ in 0..n {
            last = sut.allocate(device);
        }
        last
    }

    #[test]
    fn allocate_bytes() {
        let (mut device, mut sut) = get_sut();
        sut.allocate(&mut device).unwrap();

        let mut out = [0; 4];
        sut.allocate_bytes(&mut device, 1, &mut out).unwrap();
        assert_eq!([1, 0, 0, 0], out);

        sut.allocate_bytes(&mut device, 128, &mut out).unwrap();
        assert_eq!([2, 0, 0, 0], out);

        sut.allocate_bytes(&mut device, 512, &mut out).unwrap();
        assert_eq!([3, 0, 0, 0], out);

        sut.allocate_bytes(&mut device, 1500, &mut out).unwrap();
        assert_eq!([4, 5, 6, 0], out);
    }

    #[test]
    fn allocate() {
        let (mut device, mut sut) = get_sut();

        assert_eq!(Ok(8192), sut.count_free_addresses(&mut device));
        assert_eq!(Ok(0), sut.allocate(&mut device));
        assert_eq!(Ok(8191), sut.count_free_addresses(&mut device));

        assert_eq!(Ok(8191), take_nth_blocks(&mut sut, &mut device, 8191));
        assert_eq!(Ok(0), sut.count_free_addresses(&mut device));
    }

    #[test]
    fn release() {
        let (mut device, mut sut) = get_sut();

        assert_eq!(Ok(8191), take_nth_blocks(&mut sut, &mut device, 8192));
        assert_eq!(Ok(0), sut.count_free_addresses(&mut device));

        assert_eq!(Ok(()), sut.release(&mut device, 4000));
        assert_eq!(Ok(()), sut.release(&mut device, 5000));
        assert_eq!(Ok(()), sut.release(&mut device, 6000));
        assert_eq!(Ok(3), sut.count_free_addresses(&mut device));

        assert_eq!(Ok(4000), sut.allocate(&mut device));
        assert_eq!(Ok(5000), sut.allocate(&mut device));
        assert_eq!(Ok(6000), sut.allocate(&mut device));
    }

    #[test]
    fn allocate_n() {
        let (mut device, mut sut) = get_sut();

        assert_eq!(Ok(8191), take_nth_blocks(&mut sut, &mut device, 8192));
        assert_eq!(Ok(0), sut.count_free_addresses(&mut device));

        let mut addrs = [0; 10];
        assert_eq!(Err(Error::StorageFull), sut.allocate_n(&mut device, 8, &mut addrs));

        // Release sparse addresses
        assert_eq!(Ok(()), sut.release(&mut device, 100));
        assert_eq!(Ok(()), sut.release(&mut device, 200));
        assert_eq!(Ok(()), sut.release(&mut device, 300));
        assert_eq!(Ok(()), sut.release(&mut device, 1000));
        assert_eq!(Ok(()), sut.release(&mut device, 2000));
        assert_eq!(Ok(()), sut.release(&mut device, 3000));
        assert_eq!(Ok(()), sut.release(&mut device, 7500));
        assert_eq!(Ok(()), sut.release(&mut device, 1300));

        assert_eq!(Ok(()), sut.allocate_n(&mut device, 8, &mut addrs));
        assert_eq!([100, 200, 300, 1000, 1300, 2000, 3000, 7500], addrs[0..8]);

        // Now reproduce a rollback
        addrs[0..8].iter().for_each(|n| sut.release(&mut device, *n).unwrap());

        assert_eq!(Ok(8), sut.count_free_addresses(&mut device));
        assert_eq!(Err(Error::StorageFull), sut.allocate_n(&mut device, 10, &mut addrs));
        assert_eq!(Ok(8), sut.count_free_addresses(&mut device));
    }
}
