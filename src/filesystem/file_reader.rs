use crate::{
    BlockDevice, Error,
    filesystem::{block::Block, cache::BlockCache, layout::Layout, node::Node},
};

pub struct FileReader<'dev, D>
where
    D: BlockDevice,
{
    device: &'dev mut BlockCache<D>,
    node: Node,
}

impl<'dev, D> FileReader<'dev, D>
where
    D: BlockDevice,
{
    pub const fn new(device: &'dev mut BlockCache<D>, node: Node) -> Self {
        Self { device, node }
    }

    pub const fn file_len(&self) -> u16 {
        self.node.file_len()
    }

    pub fn read(&mut self, out: &mut [u8]) -> Result<usize, Error> {
        if out.len() < self.node.file_len() as usize {
            return Err(Error::BufferTooSmall {
                expected: self.node.file_len() as usize,
                found: out.len(),
            });
        }

        let mut block = Block::new();
        let mut from = 0;
        let blocks_needed = self.node.file_len().div_ceil(Block::LEN as u16) as usize;
        for (i, data_addr) in self.node.data_addrs().iter().take(blocks_needed).enumerate() {
            let sector = Layout::DATA.nth(*data_addr);
            self.device.read(sector, &mut block)?;
            if i == blocks_needed - 1 {
                let remaining_bytes = self.node.file_len() as usize % Block::LEN;
                out[from..from + remaining_bytes].copy_from_slice(&block[..remaining_bytes]);
            } else {
                out[from..from + Block::LEN].copy_from_slice(&block);
                from += Block::LEN;
            }
        }
        Ok(self.node.file_len() as usize)
    }
}
