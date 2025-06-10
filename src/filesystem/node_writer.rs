use crate::filesystem::{Addr, EraseFromDevice, Serializable, WriteToDevice};
use crate::io::Writer;
use crate::{BlockDevice, Error};

use super::{Block, Node};

pub struct NodeWriter<'a> {
    addr: Addr,
    node: &'a Node,
}

impl<'a> NodeWriter<'a> {
    pub const fn new(addr: Addr, node: &'a Node) -> Self {
        Self { addr, node }
    }

    const fn byte_offset(&self) -> usize {
        (self.addr as usize % Node::NODES_PER_BLOCK) * Node::SERIALIZED_SIZE
    }

    const fn sector(&self) -> Addr {
        Node::RANGE.nth(self.addr / Node::NODES_PER_BLOCK as Addr)
    }
}

impl<D> WriteToDevice<D> for NodeWriter<'_>
where
    D: BlockDevice,
{
    fn write_to_device(&self, out: &mut D) -> Result<(), Error> {
        let (sector, offset) = (self.sector(), self.byte_offset());

        let mut block = Block::new();
        out.read_block(sector, &mut block)?;

        let mut writer = Writer::new(&mut block[offset..]);
        self.node.serialize(&mut writer)?;

        out.write_block(sector, &block)
    }
}

impl<D> EraseFromDevice<D> for NodeWriter<'_>
where
    D: BlockDevice,
{
    fn erase_from_device(&self, out: &mut D) -> Result<(), Error> {
        let (sector, offset) = (self.sector(), self.byte_offset());
        let mut block = Block::new();
        out.read_block(sector, &mut block)?;
        block[offset..offset + Node::SERIALIZED_SIZE].fill(0);
        out.write_block(sector, &block)
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
        assert_eq!(Ok(()), node.serialize(&mut writer));

        out.assert_write(0, Node::RANGE.nth(0), &expected_data);
    }

    #[test]
    fn write_to_device_next_block() {
        let mut out = MockDevice::new();
        let node = &Node::new(1024, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        // At Node::NODE_SIZE bytes per node... Block 12 should be written to 2nd sector.
        let sut = NodeWriter::new(12, node);
        assert_eq!(Ok(()), sut.write_to_device(&mut out));

        let mut expected_data = [0u8; Block::LEN];
        let mut writer = Writer::new(&mut expected_data);
        assert_eq!(Ok(()), node.serialize(&mut writer));

        out.assert_write(0, Node::RANGE.nth(1), &expected_data);
    }

    #[test]
    fn write_to_device_with_offset() {
        let mut out = MockDevice::new();
        let node = &Node::new(1024, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        // At Node::NODE_SIZE bytes per node... Node 26th should be written to 3rd sector with the corresponding byte alignment.
        let sut = NodeWriter::new(26, node);
        assert_eq!(Ok(()), sut.write_to_device(&mut out));

        let mut expected_data = [0u8; Block::LEN];
        let mut writer = Writer::new(&mut expected_data[2 * Node::SERIALIZED_SIZE..]);
        assert_eq!(Ok(()), node.serialize(&mut writer));

        out.assert_write(0, Node::RANGE.nth(2), &expected_data);
    }

    #[test]
    fn erase_from_device() {
        let mut out = MockDevice::new();
        let node = &Node::new(1024, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        let sut = NodeWriter::new(0, node);
        assert_eq!(Ok(()), sut.erase_from_device(&mut out));
        out.assert_write(0, Node::RANGE.nth(0), &[0u8; Block::LEN]);
    }
}
