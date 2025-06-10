use std::io::{self, SeekFrom};

use crate::{BLOCK_SIZE, Meta, alloc_block_buffer, serde::Deserializable};

pub struct Reader {}

impl Reader {
    pub fn read_metadata<T: io::Read + io::Seek>(input: &mut T) -> io::Result<Meta> {
        let mut buf = alloc_block_buffer();
        Self::read_block(input, 0, &mut buf)?;

        Meta::deserialize(&buf)
    }

    pub(super) fn read_block<T>(
        input: &mut T,
        block_number: u32,
        out: &mut [u8],
    ) -> io::Result<usize>
    where
        T: io::Read + io::Seek,
    {
        input.seek(SeekFrom::Start(block_number as u64 * BLOCK_SIZE as u64))?;
        input.read(out)
    }
}
