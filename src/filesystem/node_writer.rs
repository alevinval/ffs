use crate::{
    BlockDevice, Error,
    filesystem::{
        Addr, Block, EraseFromDevice, Layout, Node, ReadFromDevice, Serializable,
        StaticReadFromDevice, WriteToDevice,
    },
    io::Writer,
};

const fn byte_offset(addr: Addr) -> usize {
    (addr as usize % Node::NODES_PER_BLOCK) * Node::SERIALIZED_LEN
}

const fn sector(addr: Addr) -> Addr {
    Layout::NODE.nth(addr / Node::NODES_PER_BLOCK as Addr)
}

pub struct NodeWriter<'a> {
    addr: Addr,
    node: &'a Node,
}

impl<'a> NodeWriter<'a> {
    pub const fn new(addr: Addr, node: &'a Node) -> Self {
        Self { addr, node }
    }
}

impl<D> WriteToDevice<D> for NodeWriter<'_>
where
    D: BlockDevice,
{
    fn write_to_device(&self, out: &mut D) -> Result<(), Error> {
        let (sector, offset) = (sector(self.addr), byte_offset(self.addr));

        let mut block = Block::new();
        out.read_block(sector, &mut block)?;

        let mut writer = Writer::new(&mut block[offset..]);
        self.node.serialize(&mut writer)?;

        out.write_block(sector, &block)
    }
}

pub struct NodeRef {
    addr: Addr,
}

impl NodeRef {
    pub const fn new(addr: Addr) -> Self {
        Self { addr }
    }
}

impl<D> EraseFromDevice<D> for NodeRef
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

impl<D> ReadFromDevice<D> for NodeRef
where
    D: BlockDevice,
{
    type Item = Node;

    fn read_from_device(&self, device: &mut D) -> Result<Self::Item, Error> {
        let (sector, offset) = (sector(0), byte_offset(0));
        let mut block = Block::new();
        device.read_block(sector, &mut block)?;
        let buf = &block[offset..offset + Node::SERIALIZED_LEN];
        Node::deserialize(buf)
    }
}

#[cfg(test)]
mod test {
    use crate::test_utils::MockDevice;

    use super::*;

    #[test]
    fn write_to_device() {
        let mut out = MockDevice::new();
        let node = &Node::new(1024, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        let sut = NodeWriter::new(0, node);
        assert_eq!(Ok(()), sut.write_to_device(&mut out));

        let mut expected_data = [0u8; Block::LEN];
        let mut writer = Writer::new(&mut expected_data);
        assert_eq!(Ok(42), node.serialize(&mut writer));

        out.assert_write(0, Layout::NODE.nth(0), &expected_data);
    }

    #[test]
    fn write_to_device_next_block() {
        let mut out = MockDevice::new();
        let node = &Node::new(1024, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        let sut = NodeWriter::new(12, node);
        assert_eq!(Ok(()), sut.write_to_device(&mut out));

        let mut expected_data = [0u8; Block::LEN];
        let mut writer = Writer::new(&mut expected_data);
        assert_eq!(Ok(42), node.serialize(&mut writer));

        out.assert_write(0, Layout::NODE.nth(1), &expected_data);
    }

    #[test]
    fn write_to_device_with_offset() {
        let mut out = MockDevice::new();
        let node = &Node::new(1024, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        let sut = NodeWriter::new(26, node);
        assert_eq!(Ok(()), sut.write_to_device(&mut out));

        let mut expected_data = [0u8; Block::LEN];
        let mut writer = Writer::new(&mut expected_data[2 * Node::SERIALIZED_LEN..]);
        assert_eq!(Ok(42), node.serialize(&mut writer));

        out.assert_write(0, Layout::NODE.nth(2), &expected_data);
    }

    #[test]
    fn erase_from_device() {
        let mut out = MockDevice::new();
        let sut = NodeRef::new(0);
        assert_eq!(Ok(()), sut.erase_from_device(&mut out));
        out.assert_write(0, Layout::NODE.nth(0), &[0u8; Block::LEN]);
    }
}
