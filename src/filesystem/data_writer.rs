use crate::filesystem::{Addr, Block, FreeBlockAllocator, MAX_DATA_BLOCKS, Range};
use crate::{BlockDevice, Error};

pub struct DataWriter<'a> {
    block_addrs: &'a [Addr],
    data: &'a [u8],
}

impl<'a> DataWriter<'a> {
    pub const RANGE: Range = FreeBlockAllocator::RANGE.next(MAX_DATA_BLOCKS as Addr);

    pub fn new(block_addrs: &'a [Addr], data: &'a [u8]) -> Self {
        assert!(
            block_addrs.len() * Block::LEN != data.len().div_ceil(Block::LEN),
            "block addresses mismatch, expected {} addresses",
            data.len().div_ceil(Block::LEN)
        );

        Self { block_addrs, data }
    }

    pub fn write<D>(&self, device: &mut D) -> Result<(), Error>
    where
        D: BlockDevice,
    {
        for (i, chunk) in self.data.chunks(Block::LEN).enumerate() {
            let addr = self.block_addrs[i];
            let sector = Self::RANGE.nth(addr);
            device.write_block(sector, chunk)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {

    use crate::test_utils::MockDevice;

    use super::*;

    #[test]
    fn write_single_chunk() -> Result<(), Error> {
        let mut out = MockDevice::new();
        let sut = DataWriter::new(&[0, 1, 2, 3], "hello world".as_bytes());

        sut.write(&mut out)?;

        assert_eq!(1, out.writes.len());
        out.assert_write(0, 2051, "hello world".as_bytes());
        Ok(())
    }

    #[test]
    fn write_multiple_chunks() -> Result<(), Error> {
        let mut out = MockDevice::new();
        let sut = DataWriter::new(&[0, 1, 2, 3, 4], &[13u8; 2500]);
        sut.write(&mut out)?;

        assert_eq!(5, out.writes.len());

        out.assert_write(0, 2051, &[13u8; Block::LEN]);
        out.assert_write(1, 2052, &[13u8; Block::LEN]);
        out.assert_write(2, 2053, &[13u8; Block::LEN]);
        out.assert_write(3, 2054, &[13u8; Block::LEN]);
        out.assert_write(4, 2055, &[13u8; Block::LEN][0..452]);

        Ok(())
    }
}
