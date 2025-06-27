use crate::{
    Error,
    filesystem::{Addr, Block, SerdeLen, Serializable},
    io::Write,
};

const fn get_blocks_needed(file_size: u16) -> usize {
    (file_size as usize).div_ceil(Block::LEN)
}

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

    /// The number of nodes that fit in a single block.
    pub const NODES_PER_BLOCK: usize = Block::LEN / Self::SERDE_LEN;

    pub const fn new(file_size: u16, block_addrs: [Addr; Self::BLOCKS_PER_NODE]) -> Self {
        Self { file_len: file_size, block_addrs }
    }

    pub fn file_len(&self) -> u16 {
        self.file_len
    }

    pub fn block_addrs(&self) -> &[Addr] {
        &self.block_addrs
    }

    pub fn deserialize(buf: &[u8]) -> Result<Node, Error> {
        let mut file_size = [0u8; 2];
        file_size.copy_from_slice(&buf[0..2]);
        let file_size = u16::from_le_bytes(file_size);

        let n_blocks = get_blocks_needed(file_size);
        let mut block_addrs = [0u32; Node::BLOCKS_PER_NODE];
        let mut block_addr_buf = [0u8; 4];
        let mut n = 2;
        for addr in block_addrs.iter_mut().take(n_blocks) {
            block_addr_buf.copy_from_slice(&buf[n..n + 4]);
            n += 4;
            *addr = Addr::from_le_bytes(block_addr_buf);
        }

        Ok(Node { file_len: file_size, block_addrs })
    }
}

impl SerdeLen for Node {
    const SERDE_LEN: usize = 2 + (4 * Self::BLOCKS_PER_NODE);
}

impl Serializable for Node {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<usize, Error> {
        let mut n = writer.write_u16(self.file_len)?;
        for addr in self.block_addrs() {
            n += writer.write_u32(*addr)?;
        }
        Ok(n)
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn constants() {
        assert_eq!(12, Node::NODES_PER_BLOCK);
    }

    #[test]
    fn getters() {
        let sut = Node::new(123, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        assert_eq!(123, sut.file_len());
        assert_eq!(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10], sut.block_addrs());
    }

    #[test]
    fn serde_symmetry() {
        let mut block = Block::new();

        let expected = Node::new(5120, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        assert_eq!(Ok(Node::SERDE_LEN), expected.serialize(&mut block.writer()));
        let actual = Node::deserialize(&block).unwrap();

        assert_eq!(expected, actual);
    }
}
