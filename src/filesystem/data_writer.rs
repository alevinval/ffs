use crate::{
    BlockDevice, Error,
    filesystem::{Addr, Block, Layout, WriteToDevice},
};

pub struct DataWriter<'a> {
    block_addrs: &'a [Addr],
    data: &'a [u8],
}

impl<'a> DataWriter<'a> {
    pub fn new(block_addrs: &'a [Addr], data: &'a [u8]) -> Self {
        assert!(
            block_addrs.len() * Block::LEN != data.len().div_ceil(Block::LEN),
            "block addresses mismatch, expected {} addresses",
            data.len().div_ceil(Block::LEN)
        );

        Self { block_addrs, data }
    }
}

impl<D> WriteToDevice<D> for DataWriter<'_>
where
    D: BlockDevice,
{
    fn write_to_device(&self, device: &mut D) -> Result<(), Error> {
        for (i, chunk) in self.data.chunks(Block::LEN).enumerate() {
            let addr = self.block_addrs[i];
            let sector = Layout::DATA.nth(addr);
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

        sut.write_to_device(&mut out)?;

        assert_eq!(1, out.writes.len());
        out.assert_write(0, Layout::DATA.nth(0), "hello world".as_bytes());
        Ok(())
    }

    #[test]
    fn write_multiple_chunks() -> Result<(), Error> {
        let mut out = MockDevice::new();
        let sut = DataWriter::new(&[0, 1, 2, 3, 4], &[13u8; 2500]);
        sut.write_to_device(&mut out)?;

        assert_eq!(5, out.writes.len());

        out.assert_write(0, Layout::DATA.nth(0), &[13u8; Block::LEN]);
        out.assert_write(1, Layout::DATA.nth(1), &[13u8; Block::LEN]);
        out.assert_write(2, Layout::DATA.nth(2), &[13u8; Block::LEN]);
        out.assert_write(3, Layout::DATA.nth(3), &[13u8; Block::LEN]);
        out.assert_write(4, Layout::DATA.nth(4), &[13u8; Block::LEN][0..452]);

        Ok(())
    }
}
