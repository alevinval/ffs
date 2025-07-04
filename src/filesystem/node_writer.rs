use crate::{
    BlockDevice, Error,
    filesystem::{Addr, Block, Layout, Node, Serializable, Store},
};

pub struct NodeWriter<'a> {
    addr: Addr,
    node: &'a Node,
}

impl<'a> NodeWriter<'a> {
    pub const fn new(addr: Addr, node: &'a Node) -> Self {
        Self { addr, node }
    }
}

impl<D> Store<D> for NodeWriter<'_>
where
    D: BlockDevice,
{
    fn store(&self, device: &mut D) -> Result<(), Error> {
        let mut block = Block::new();
        self.node.serialize(&mut block.writer())?;
        device.write(Layout::NODE.nth(self.addr), &block)
    }
}

#[cfg(test)]
mod test {
    use crate::test_utils::MockDevice;

    use super::*;

    #[test]
    fn write_to_device() {
        let mut device = MockDevice::new();
        let node = &Node::new(1024, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        let sut = NodeWriter::new(123, node);
        assert_eq!(Ok(()), sut.store(&mut device));

        let mut expected = Block::new();
        assert_eq!(Ok(42), node.serialize(&mut expected.writer()));

        device.assert_write(0, Layout::NODE.nth(123), &expected);
    }
}
