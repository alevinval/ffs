use crate::{
    BlockDevice, Error,
    filesystem::{Addr, Addressable, Serializable, block::Block, layout::Layout},
};

pub fn store<D, T>(device: &mut D, logical_addr: Addr, object: &T) -> Result<(), Error>
where
    D: BlockDevice,
    T: Serializable + Addressable,
{
    let mut block = Block::new();
    let mut writer = block.writer();
    object.serialize(&mut writer)?;
    device.write(T::LAYOUT.nth(logical_addr), &block)
}

pub fn store_data<D>(device: &mut D, block_addrs: &[Addr], data: &[u8]) -> Result<(), Error>
where
    D: BlockDevice,
{
    assert!(
        block_addrs.len() >= data.len().div_ceil(Block::LEN),
        "block addresses mismatch, found {} but expected {}",
        block_addrs.len(),
        data.len().div_ceil(Block::LEN)
    );

    for (i, chunk) in data.chunks(Block::LEN).enumerate() {
        let addr = block_addrs[i];
        let sector = Layout::DATA.nth(addr);
        device.write(sector, chunk)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {

    use crate::test_utils::MockDevice;

    use super::*;

    #[test]
    #[should_panic(expected = "block addresses mismatch, found 3 but expected 4")]
    fn test_store_data_less_addrs_than_chunks_panics() {
        let mut device = MockDevice::new();
        let _ = store_data(&mut device, &[0, 1, 2], &[0; 1537]); // 4 blocks, 3 addrs
    }

    #[test]
    fn test_store_data_single_chunk() {
        let mut device = MockDevice::new();
        assert_eq!(Ok(()), store_data(&mut device, &[0], b"hello world"));
        assert_eq!(1, device.writes.len());
        device.assert_write(0, Layout::DATA.nth(0), b"hello world");
    }

    #[test]
    fn test_store_data_multiple_chunks() {
        let mut device = MockDevice::new();
        assert_eq!(Ok(()), store_data(&mut device, &[0, 1, 2, 3, 4], &[13u8; 2500]));
        assert_eq!(5, device.writes.len());
        device.assert_write(0, Layout::DATA.nth(0), &[13u8; Block::LEN]);
        device.assert_write(1, Layout::DATA.nth(1), &[13u8; Block::LEN]);
        device.assert_write(2, Layout::DATA.nth(2), &[13u8; Block::LEN]);
        device.assert_write(3, Layout::DATA.nth(3), &[13u8; Block::LEN]);
        device.assert_write(4, Layout::DATA.nth(4), &[13u8; Block::LEN][0..452]);
    }
}
