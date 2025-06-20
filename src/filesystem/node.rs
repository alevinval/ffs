use crate::{
    Error,
    filesystem::{Addr, Block, File, MAX_FILES, Range, Serializable},
    io::Write,
};

const fn get_blocks_needed(file_size: u16) -> usize {
    (file_size as usize).div_ceil(Block::LEN)
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Node {
    file_len: u16,
    block_addrs: [Addr; Node::BLOCKS],
}

impl Node {
    /// The number of data blocks a single file node can reference.
    /// This limits the maximum file size and is used for serialization, allocation, and layout.
    pub const BLOCKS: usize = 10;

    /// The maximum file size (in bytes) that a single node can represent.
    pub const MAX_FILE_SIZE: usize = Self::BLOCKS * Block::LEN;

    /// The range of sectors on disk where nodes are stored.
    pub const RANGE: Range = File::RANGE.next(MAX_FILES as Addr);

    /// The size in bytes of a serialized Node structure.
    /// 2 bytes for file_len + 4 bytes per block address.
    pub const SERIALIZED_SIZE: usize = 2 + (Self::BLOCKS * 4);

    /// The number of nodes that fit in a single block.
    pub const NODES_PER_BLOCK: usize = Block::LEN / Self::SERIALIZED_SIZE;

    pub const fn new(file_size: u16, block_addrs: [Addr; Self::BLOCKS]) -> Self {
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
        let mut block_addrs = [0u32; Node::BLOCKS];
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

impl Serializable for Node {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        writer.write_u16(self.file_len)?;
        for addr in self.block_addrs().iter().take(get_blocks_needed(self.file_len)) {
            writer.write_u32(*addr)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn constants() {
        assert_eq!(42, Node::SERIALIZED_SIZE);
        assert_eq!(12, Node::NODES_PER_BLOCK);
    }

    #[test]
    fn getters() {
        let sut = Node::new(123, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        assert_eq!(123, sut.file_len());
        assert_eq!(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10], sut.block_addrs());
    }

    #[test]
    fn serde_symmetry() -> Result<(), Error> {
        let mut block = Block::new();
        let expected = Node::new(5120, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        expected.serialize(&mut block.writer())?;
        let actual = Node::deserialize(&block)?;

        assert_eq!(expected, actual);
        Ok(())
    }
}
