use crate::{
    BLOCK_SIZE, Error, Index, MAX_BLOCKS_PER_NODE,
    serde::{Deserializable, Serializable},
};

const fn get_blocks_needed(file_size: u16) -> usize {
    (file_size as usize).div_ceil(BLOCK_SIZE)
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Node {
    // File size in bytes
    file_size: u16,

    // Block indexes that this node points to
    block_indexes: [Index; MAX_BLOCKS_PER_NODE],
}

impl Node {
    pub const NODE_SIZE: usize = 2 + (MAX_BLOCKS_PER_NODE * 4);
    pub const NODES_PER_BLOCK: usize = BLOCK_SIZE / Self::NODE_SIZE;

    pub const fn new(file_size: u16, block_indexes: [Index; MAX_BLOCKS_PER_NODE]) -> Self {
        Self { file_size, block_indexes }
    }

    pub const fn get_file_size(&self) -> usize {
        self.file_size as usize
    }

    pub const fn get_block_indexes(&self) -> &[Index] {
        &self.block_indexes
    }
}

impl Serializable for Node {
    fn serialize(&self, out: &mut [u8]) -> Result<usize, Error> {
        out[0..2].copy_from_slice(&self.file_size.to_le_bytes());
        let mut n = 2;
        for block_index in self.get_block_indexes().iter().take(get_blocks_needed(self.file_size)) {
            out[n..n + 4].copy_from_slice(&block_index.to_le_bytes());
            n += 4;
        }
        Ok(n)
    }
}

impl Deserializable<Node> for Node {
    fn deserialize(buf: &[u8]) -> Result<Node, Error> {
        let mut file_size = [0u8; 2];
        file_size.copy_from_slice(&buf[0..2]);
        let file_size = u16::from_le_bytes(file_size);

        let n_blocks = get_blocks_needed(file_size);
        let mut block_indexes = [0u32; 10];
        let mut block_index_buf = [0u8; 4];
        let mut n = 2;
        for block_index in block_indexes.iter_mut().take(n_blocks) {
            block_index_buf.copy_from_slice(&buf[n..n + 4]);
            n += 4;
            *block_index = Index::from_le_bytes(block_index_buf);
        }

        Ok(Node { file_size, block_indexes })
    }
}

#[cfg(test)]
mod test {
    use crate::alloc_block_buffer;

    use super::*;
    #[test]
    fn constants() {
        assert_eq!(42, Node::NODE_SIZE);
        assert_eq!(12, Node::NODES_PER_BLOCK);
    }

    #[test]
    fn getters() {
        let sut = Node::new(123, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        assert_eq!(123, sut.get_file_size());
        assert_eq!(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10], sut.get_block_indexes());
    }

    #[test]
    fn serde_symmetry() -> Result<(), Error> {
        let mut buf = alloc_block_buffer();

        let expected = Node::new(5120, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        expected.serialize(&mut buf)?;
        let actual = Node::deserialize(&buf)?;

        assert_eq!(expected, actual);
        Ok(())
    }
}
