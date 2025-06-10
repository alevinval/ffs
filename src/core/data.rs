use std::io::{self, Cursor, Write};

use crate::{Index, serde::Serializable};

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
    fn serialize(&self, buf: &mut [u8]) -> io::Result<usize> {
        let mut cursor = Cursor::new(buf);
        cursor.write(self.data)
    }
}
