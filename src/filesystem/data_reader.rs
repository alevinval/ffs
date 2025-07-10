use crate::{
    BlockDevice, Error,
    filesystem::{block::Block, cache::BlockCache, layout::Layout, node::Node},
    io::Writer,
};

pub struct DataReader<'dev, D>
where
    D: BlockDevice,
{
    device: &'dev mut BlockCache<D>,
    node: Node,
}

impl<'dev, D> DataReader<'dev, D>
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
        let mut writer = Writer::new(out);
        let blocks_needed = self.node.file_len().div_ceil(Block::LEN as u16) as usize;
        for (i, data_addr) in self.node.data_addrs().iter().take(blocks_needed).enumerate() {
            let sector = Layout::DATA.nth(*data_addr);
            self.device.read(sector, &mut block)?;
            if i == blocks_needed - 1 {
                let remaining_bytes = self.node.file_len() as usize % Block::LEN;
                writer.write(&block[..remaining_bytes])?;
            } else {
                writer.write(&block)?;
            }
        }
        Ok(self.node.file_len() as usize)
    }
}
