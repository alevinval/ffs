use crate::{
    Addr, BlockDevice, Error,
    filesystem::{
        Block, EraseFromDevice, Layout, ReadFromDevice, node::Node, node_writer::NodeWriter,
    },
};

const fn byte_offset(addr: Addr) -> usize {
    (addr as usize % Node::NODES_PER_BLOCK) * Node::SERIALIZED_LEN
}

const fn sector(addr: Addr) -> Addr {
    Layout::NODE.nth(addr / Node::NODES_PER_BLOCK as Addr)
}

pub struct NodeHandle {
    addr: Addr,
}

impl NodeHandle {
    pub const fn new(addr: Addr) -> Self {
        Self { addr }
    }

    pub const fn writer<'a>(&'a self, node: &'a Node) -> NodeWriter<'a> {
        NodeWriter::new(self.addr, node)
    }
}

impl<D> ReadFromDevice<D> for NodeHandle
where
    D: BlockDevice,
{
    type Item = Node;

    fn read_from_device(&self, device: &mut D) -> Result<Self::Item, Error> {
        let (sector, offset) = (sector(self.addr), byte_offset(self.addr));
        let mut block = Block::new();
        device.read_block(sector, &mut block)?;
        Node::deserialize(&block[offset..])
    }
}

impl<D> EraseFromDevice<D> for NodeHandle
where
    D: BlockDevice,
{
    fn erase_from_device(&self, out: &mut D) -> Result<(), Error> {
        let (sector, offset) = (sector(self.addr), byte_offset(self.addr));
        let mut block = Block::new();
        out.read_block(sector, &mut block)?;
        block[offset..offset + Node::SERIALIZED_LEN].fill(0);
        out.write_block(sector, &block)
    }
}

#[cfg(test)]
mod test {

    use crate::{filesystem::WriteToDevice, test_utils::MockDevice};

    use super::*;

    #[test]
    fn read_from_device() -> Result<(), Error> {
        let expected = Node::new(5084, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);

        let mut device = MockDevice::new();
        let handle = NodeHandle::new(15);
        handle.writer(&expected).write_to_device(&mut device)?;
        let actual = handle.read_from_device(&mut device)?;
        assert_eq!(expected, actual);

        Ok(())
    }

    #[test]
    fn erase_from_device() -> Result<(), Error> {
        let mut out = MockDevice::new();
        let sut = NodeHandle::new(15);
        sut.erase_from_device(&mut out)?;
        out.assert_write(0, Layout::NODE.nth(1), &[0; Block::LEN]);
        Ok(())
    }
}
