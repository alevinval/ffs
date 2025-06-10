use crate::{Error, Index, serde::Serializable};

pub struct Data<'a> {
    block_index: Index,
    data: &'a [u8],
}

impl<'a> Data<'a> {
    pub const fn new(block_index: Index, data: &'a [u8]) -> Self {
        Self { block_index, data }
    }

    pub const fn get_block_index(&self) -> Index {
        self.block_index
    }
}

impl Serializable for Data<'_> {
    fn serialize(&self, buf: &mut [u8]) -> Result<usize, Error> {
        buf[0..self.data.len()].copy_from_slice(self.data);
        Ok(512)
    }
}
