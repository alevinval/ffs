use crate::{
    Addr, BlockDevice, Deserializable, DeviceAddr, Error, FixedLen, Serializable,
    block::Block,
    device_layout::DeviceLayout,
    io::{Reader, Writer},
};

/// Length of the buffer used to store/load data from the block device.
///
/// Must be able to fit the largest structure of the library. At the moment
/// that's 3 blocks size.
///
/// TODO: When Rust supports associated const variables, move to [`SerdeLen`] trait and
/// have it calculated automatically for each type.
const BUFFER_LEN: usize = Block::LEN * 3;

pub fn store<D, T>(device: &mut D, logical: Addr, object: &T) -> Result<(), Error>
where
    D: BlockDevice,
    T: DeviceAddr + Serializable,
{
    assert!(T::BLOCKS_LEN <= 3, "nothing should serialize to more than 3 blocks");
    let mut buffer = [0u8; BUFFER_LEN];
    let mut writer = Writer::new(&mut buffer);
    object.serialize(&mut writer)?;

    for (offset, chunk) in buffer.chunks(Block::LEN).take(T::BLOCKS_LEN).enumerate() {
        device.write(T::addr(logical, offset), chunk)?;
    }
    Ok(())
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
        device.write(DeviceLayout::DATA.nth(addr), chunk)?;
    }
    Ok(())
}

pub fn load<D, T>(device: &mut D, logical: Addr) -> Result<T, Error>
where
    D: BlockDevice,
    T: DeviceAddr + Deserializable<T>,
{
    assert!(T::BLOCKS_LEN <= 3, "nothing should serialize to more than 3 blocks");
    let mut buffer = [0u8; BUFFER_LEN];
    for (offset, chunk) in buffer.chunks_mut(Block::LEN).take(T::BLOCKS_LEN).enumerate() {
        device.read(T::addr(logical, offset), chunk)?;
    }
    let mut reader = Reader::new(&buffer);
    T::deserialize(&mut reader)
}

pub fn erase<D, T>(device: &mut D, logical: Addr) -> Result<(), Error>
where
    D: BlockDevice,
    T: DeviceAddr + FixedLen,
{
    let empty_block = Block::new();
    for offset in 0..T::BLOCKS_LEN {
        device.write(T::addr(logical, offset), &empty_block)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {

    use crate::testutils::MockDevice;

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
        device.assert_write(0, DeviceLayout::DATA.nth(0), b"hello world");
    }

    #[test]
    fn test_store_data_multiple_chunks() {
        let mut device = MockDevice::new();
        assert_eq!(Ok(()), store_data(&mut device, &[0, 1, 2, 3, 4], &[13u8; 2500]));
        assert_eq!(5, device.writes.len());
        device.assert_write(0, DeviceLayout::DATA.nth(0), &[13u8; Block::LEN]);
        device.assert_write(1, DeviceLayout::DATA.nth(1), &[13u8; Block::LEN]);
        device.assert_write(2, DeviceLayout::DATA.nth(2), &[13u8; Block::LEN]);
        device.assert_write(3, DeviceLayout::DATA.nth(3), &[13u8; Block::LEN]);
        device.assert_write(4, DeviceLayout::DATA.nth(4), &[13u8; 452]);
    }
}
