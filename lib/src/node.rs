use crate::{
    Addr, Block, Deserializable, DeviceAddr, Error, FixedLen, Serializable, constants,
    device_layout::DeviceLayout,
    io::{Read, Write},
};

const N: usize = constants::NODE_DATA_BLOCKS_LEN;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Node {
    file_len: u16,
    data_addrs: [Addr; N],
}

impl Node {
    #[must_use]
    pub const fn new(file_size: u16, data_addrs: [Addr; N]) -> Self {
        Self { file_len: file_size, data_addrs }
    }

    #[must_use]
    pub const fn data_addrs(&self) -> &[Addr] {
        &self.data_addrs
    }

    #[must_use]
    pub const fn file_len(&self) -> u16 {
        self.file_len
    }

    #[must_use]
    pub const fn blocks_needed(&self) -> usize {
        (self.file_len as usize).div_ceil(Block::LEN)
    }
}

impl DeviceAddr for Node {
    const LAYOUT: DeviceLayout = DeviceLayout::NODE;
}

impl FixedLen for Node {
    const BYTES_LEN: usize = 2 + (size_of::<Addr>() * N);
}

impl Serializable for Node {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<usize, Error> {
        let mut n = writer.write_u16(self.file_len)?;
        for addr in self.data_addrs() {
            n += writer.write_addr(*addr)?;
        }
        Ok(n)
    }
}

impl Deserializable<Self> for Node {
    fn deserialize<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let file_len = reader.read_u16()?;
        let mut block_addrs = [0 as Addr; constants::NODE_DATA_BLOCKS_LEN];
        for addr in &mut block_addrs {
            *addr = reader.read_addr()?;
        }
        Ok(Self { file_len, data_addrs: block_addrs })
    }
}

#[cfg(test)]
mod tests {

    use crate::test_serde_symmetry;

    use super::*;

    test_serde_symmetry!(Node, Node::new(5120, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]));

    #[test]
    fn test_node_blocks_needed() {
        let node = Node::new(1, [0; N]);
        assert_eq!(1, node.blocks_needed());

        let node = Node::new(1024, [0; N]);
        assert_eq!(2, node.blocks_needed());

        let node = Node::new(1025, [0; N]);
        assert_eq!(3, node.blocks_needed());
    }
}
