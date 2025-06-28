use crate::{Addr, BlockDevice, Error, filesystem::block::Block};

#[derive(Debug)]
struct CacheEntry {
    sector: Addr,
    block: Block,
}

#[derive(Debug)]
pub struct BlockCache<D: BlockDevice> {
    device: D,
    cache: [Option<CacheEntry>; 8],
}

impl<D: BlockDevice> BlockCache<D> {
    pub const fn mount(device: D) -> Self {
        Self { device, cache: [const { None }; 8] }
    }

    pub fn unmount(self) -> D {
        self.device
    }

    fn get(&mut self, sector: Addr) -> Option<&mut Block> {
        if let Some(pos) =
            self.cache.iter().position(|e| e.as_ref().is_some_and(|e| e.sector == sector))
        {
            self.cache.swap(0, pos);
            return self.cache[0].as_mut().map(|e| &mut e.block);
        }
        None
    }

    fn insert(&mut self, sector: Addr, block: Block) {
        self.cache.rotate_right(1);
        self.cache[0] = Some(CacheEntry { sector, block });
    }
}

impl<D: BlockDevice> BlockDevice for BlockCache<D> {
    fn read_block(&mut self, sector: Addr, buf: &mut [u8]) -> Result<(), Error> {
        if let Some(block) = self.get(sector) {
            buf.copy_from_slice(block);
            return Ok(());
        }

        self.device.read_block(sector, buf)?;
        let block = Block::from_slice(buf);
        self.insert(sector, block);

        Ok(())
    }

    fn write_block(&mut self, sector: Addr, buf: &[u8]) -> Result<(), Error> {
        self.device.write_block(sector, buf)?;
        if let Some(block) = self.get(sector) {
            block.copy_from_slice(buf);
        }
        Ok(())
    }
}
