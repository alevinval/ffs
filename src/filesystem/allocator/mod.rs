use crate::{
    BlockDevice, Error,
    filesystem::{
        Addr, Block, Deserializable, Serializable, allocator::bitmap::AllocationBitmap,
        layout::Layout, node::Node,
    },
};

mod bitmap;

#[derive(Debug)]
pub struct Allocator {
    layout: Layout,
    last_accessed: Addr,
}

impl Allocator {
    pub const SLOTS: usize = AllocationBitmap::SLOTS;

    pub const fn new(layout: Layout) -> Self {
        Self { last_accessed: 0, layout }
    }

    pub fn count_free_addresses<D: BlockDevice>(&self, device: &mut D) -> Result<usize, Error> {
        let mut block = Block::new();
        let mut total = 0;
        for sector in self.layout.iter_sectors() {
            device.read(sector, &mut block)?;
            let bitmap = AllocationBitmap::deserialize(&mut block.reader())?;
            total += bitmap.count_free_addresses();
        }
        Ok(total)
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
    pub fn allocate_n<D: BlockDevice>(
        &mut self,
        device: &mut D,
        addrs: &mut [Addr],
        n: usize,
    ) -> Result<(), Error> {
        if addrs.len() < n {
            return Err(Error::BufferTooSmall { expected: n, found: addrs.len() });
        }

        let mut current = 0;
        while current < n {
            match self.allocate(device) {
                Ok(addr) => {
                    addrs[current] = addr;
                    current += 1;
                }
                Err(_) => {
                    for addr in addrs.iter().take(current) {
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
            device.read(sector, &mut block)?;
            let mut bitmap = AllocationBitmap::deserialize(&mut block.reader())?;

            if let Some(allocation) = bitmap.allocate() {
                bitmap.serialize(&mut block.writer())?;
                device.write(sector, &block)?;
                self.last_accessed = addr;
                return Ok(to_addr(addr, allocation));
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
    pub fn release<D: BlockDevice>(&mut self, device: &mut D, addr: Addr) -> Result<(), Error> {
        let bitmap_addr = to_bitmap_addr(addr) as Addr;
        let bitmap_sector = self.layout.nth(bitmap_addr);
        let bitmap_offset = to_bitmap_offset(addr);

        let mut block = Block::new();
        device.read(bitmap_sector, &mut block)?;

        let mut bitmap = AllocationBitmap::deserialize(&mut block.reader())?;
        bitmap.release(bitmap_offset);
        bitmap.serialize(&mut block.writer())?;

        device.write(bitmap_sector, &block)?;
        if bitmap_addr < self.last_accessed {
            self.last_accessed = bitmap_addr;
        }
        Ok(())
    }
}

/// Provides utility functions so the [`Allocator`] can work with [`Node`] and file data.
pub trait DataAllocator {
    fn allocate_node_data<D: BlockDevice>(
        &mut self,
        device: &mut D,
        file_size: usize,
    ) -> Result<Node, Error>;

    fn release_node_data<D: BlockDevice>(
        &mut self,
        device: &mut D,
        node: &Node,
    ) -> Result<(), Error>;
}

impl DataAllocator for Allocator {
    /// Attempts to allocate enough blocks to fit `file_size` bytes and returns a [`Node`] instance
    /// with all the allocated addresses.
    fn allocate_node_data<D: BlockDevice>(
        &mut self,
        device: &mut D,
        file_size: usize,
    ) -> Result<Node, Error> {
        let mut block_addrs = [0; Node::BLOCKS_PER_NODE];
        self.allocate_n(device, &mut block_addrs, file_size.div_ceil(Block::LEN))?;
        Ok(Node::new(file_size as u16, block_addrs))
    }

    /// Attempts to allocate enough blocks to fit `file_size` bytes and returns a [`Node`] instance
    /// with all the allocated addresses.
    fn release_node_data<D: BlockDevice>(
        &mut self,
        device: &mut D,
        node: &Node,
    ) -> Result<(), Error> {
        for addr in node.block_addrs() {
            self.release(device, *addr)?;
        }
        Ok(())
    }
}

const fn to_bitmap_offset(addr: Addr) -> Addr {
    addr % AllocationBitmap::SLOTS as Addr
}

const fn to_bitmap_addr(addr: Addr) -> Addr {
    addr / AllocationBitmap::SLOTS as Addr
}

const fn to_addr(bitmap_addr: Addr, allocated_addr: Addr) -> Addr {
    bitmap_addr * AllocationBitmap::SLOTS as Addr + allocated_addr
}

#[cfg(test)]
mod test {
    use crate::disk::MemoryDisk;

    use super::*;

    const TEST_LAYOUT: Layout = Layout::new(0, 2);

    fn get_sut() -> (MemoryDisk, Allocator) {
        let device = MemoryDisk::fit(TEST_LAYOUT.sector_count());
        let sut = Allocator::new(TEST_LAYOUT);
        (device, sut)
    }

    fn take_nth_blocks<D: BlockDevice>(
        sut: &mut Allocator,
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
        assert_eq!(Err(Error::StorageFull), sut.allocate_n(&mut device, &mut addrs, 8));

        // Release sparse addresses
        assert_eq!(Ok(()), sut.release(&mut device, 100));
        assert_eq!(Ok(()), sut.release(&mut device, 200));
        assert_eq!(Ok(()), sut.release(&mut device, 300));
        assert_eq!(Ok(()), sut.release(&mut device, 1000));
        assert_eq!(Ok(()), sut.release(&mut device, 2000));
        assert_eq!(Ok(()), sut.release(&mut device, 3000));
        assert_eq!(Ok(()), sut.release(&mut device, 7500));
        assert_eq!(Ok(()), sut.release(&mut device, 1300));

        assert_eq!(Ok(()), sut.allocate_n(&mut device, &mut addrs, 8));
        assert_eq!([100, 200, 300, 1000, 1300, 2000, 3000, 7500], addrs[0..8]);

        // Now reproduce a rollback
        addrs[0..8].iter().for_each(|n| sut.release(&mut device, *n).unwrap());

        assert_eq!(Ok(8), sut.count_free_addresses(&mut device));
        assert_eq!(Err(Error::StorageFull), sut.allocate_n(&mut device, &mut addrs, 10));
        assert_eq!(Ok(8), sut.count_free_addresses(&mut device));
    }

    #[test]
    fn allocate_node_data() {
        let (mut device, mut sut) = get_sut();

        let node = sut.allocate_node_data(&mut device, 1).unwrap();
        assert_eq!([0, 0, 0, 0, 0, 0, 0, 0, 0, 0], node.block_addrs());

        let node = sut.allocate_node_data(&mut device, 128).unwrap();
        assert_eq!([1, 0, 0, 0, 0, 0, 0, 0, 0, 0], node.block_addrs());

        let node = sut.allocate_node_data(&mut device, 512).unwrap();
        assert_eq!([2, 0, 0, 0, 0, 0, 0, 0, 0, 0], node.block_addrs());

        let node = sut.allocate_node_data(&mut device, 1500).unwrap();
        assert_eq!([3, 4, 5, 0, 0, 0, 0, 0, 0, 0], node.block_addrs());
    }
}
