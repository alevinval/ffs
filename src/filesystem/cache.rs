use crate::{
    BlockDevice, Error,
    filesystem::{Addr, block::Block},
};

#[derive(Debug)]
struct CacheEntry {
    sector: Addr,
    block: Block,
}

#[derive(Debug)]
pub struct BlockCache<D: BlockDevice, const SIZE: usize = 8> {
    delegate: D,
    entries: [Option<CacheEntry>; SIZE],
}

/// Implements an LRU cache for a [`BlockDevice`] to minimize the number of
/// read and write operations to the underlying device. It can be used as a
/// drop-in replacement for any [`BlockDevice`].
impl<D: BlockDevice, const SIZE: usize> BlockCache<D, SIZE> {
    /// Takes ownership of a [`BlockDevice`], and returns a new [`BlockCache`].
    pub const fn mount(device: D) -> Self {
        Self { delegate: device, entries: [const { None }; SIZE] }
    }

    /// Returns ownership of the wrapped device.
    pub fn unmount(self) -> D {
        self.delegate
    }

    fn get(&mut self, sector: Addr) -> Option<&mut Block> {
        if let Some(pos) = self
            .entries
            .iter()
            .position(|option| option.as_ref().is_some_and(|entry| entry.sector == sector))
        {
            self.entries.swap(0, pos);
            return self.entries[0].as_mut().map(|entry| &mut entry.block);
        }
        None
    }

    fn insert(&mut self, sector: Addr, block: Block) {
        self.entries.rotate_right(1);
        self.entries[0] = Some(CacheEntry { sector, block });
    }
}

/// Implements the [`BlockDevice`] trait for the [`BlockCache`]. Intercepting
/// read and write operations to read and populate the cache.
impl<D: BlockDevice> BlockDevice for BlockCache<D> {
    fn read(&mut self, sector: Addr, buf: &mut [u8]) -> Result<(), Error> {
        if let Some(block) = self.get(sector) {
            buf.copy_from_slice(block);
            return Ok(());
        }

        self.delegate.read(sector, buf)?;
        let block = Block::from_slice(buf);
        self.insert(sector, block);
        Ok(())
    }

    fn write(&mut self, sector: Addr, buf: &[u8]) -> Result<(), Error> {
        self.delegate.write(sector, buf)?;
        if let Some(block) = self.get(sector) {
            block.copy_from_slice(buf);
        }
        Ok(())
    }
}
