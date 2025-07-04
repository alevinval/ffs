use crate::{
    Error,
    filesystem::{Addr, Addressable, Block, Deserializable, Layout, SerdeLen, Serializable},
    io::{Read, Write},
};

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Node {
    file_len: u16,
    block_addrs: [Addr; Node::BLOCKS_PER_NODE],
}

impl Node {
    /// The number of data blocks a single file node can reference.
    /// This limits the maximum file size and is used for serialization, allocation, and layout.
    pub const BLOCKS_PER_NODE: usize = 10;

    /// The maximum file size (in bytes) that a single node can represent.
    pub const MAX_FILE_SIZE: usize = Self::BLOCKS_PER_NODE * Block::LEN;

    pub const fn new(file_size: u16, block_addrs: [Addr; Self::BLOCKS_PER_NODE]) -> Self {
        Self { file_len: file_size, block_addrs }
    }

    pub const fn block_addrs(&self) -> &[Addr] {
        &self.block_addrs
    }
}

impl SerdeLen for Node {
    const SERDE_LEN: usize = 2 + (size_of::<Addr>() * Self::BLOCKS_PER_NODE);
}

impl Addressable for Node {
    fn layout() -> Layout {
        Layout::NODE
    }
}

impl Serializable for Node {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<usize, Error> {
        let mut n = writer.write_u16(self.file_len)?;
        for addr in self.block_addrs() {
            n += writer.write_addr(*addr)?;
        }
        Ok(n)
    }
}

impl Deserializable<Self> for Node {
    fn deserialize<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let file_len = reader.read_u16()?;
        let mut block_addrs = [0 as Addr; Self::BLOCKS_PER_NODE];
        for addr in block_addrs.iter_mut() {
            *addr = reader.read_addr()?;
        }
        Ok(Self { file_len, block_addrs })
    }
}

#[cfg(test)]
mod test {

    use crate::test_serde_symmetry;

    use super::*;

    test_serde_symmetry!(Node, Node::new(5120, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]));
}
