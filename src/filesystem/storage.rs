use crate::{
    BlockDevice, Error,
    filesystem::{
        Addr, Addressable, Deserializable, SerdeLen, Serializable, block::Block, layouts::Layout,
    },
    io::{Reader, Writer},
};

pub fn store<D, T>(device: &mut D, logical_addr: Addr, object: &T) -> Result<(), Error>
where
    D: BlockDevice,
    T: Addressable + Serializable + SerdeLen,
{
    assert!(T::SERDE_BLOCK_COUNT <= 3, "nothing should serialize to more than 3 blocks");
    let mut buf = [0u8; Block::LEN * 3];
    let mut writer = Writer::new(&mut buf);
    object.serialize(&mut writer)?;

    let addr = T::LAYOUT.nth(logical_addr);
    for (i, chunk) in buf.chunks(Block::LEN).take(T::SERDE_BLOCK_COUNT).enumerate() {
        device.write(addr + i as Addr, chunk)?;
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
        device.write(Layout::DATA.nth(addr), chunk)?;
    }
    Ok(())
}

pub fn load<D, T>(device: &mut D, logical_addr: Addr) -> Result<T, Error>
where
    D: BlockDevice,
    T: Addressable + SerdeLen + Deserializable<T>,
{
    assert!(T::SERDE_BLOCK_COUNT <= 3, "nothing should serialize to more than 3 blocks");
    let mut buffer = [0u8; Block::LEN * 3];
    let start_sector = T::LAYOUT.nth(logical_addr);
    for (i, chunk) in buffer.chunks_mut(Block::LEN).take(T::SERDE_BLOCK_COUNT).enumerate() {
        device.read(start_sector + i as Addr, chunk)?;
    }
    let mut reader = Reader::new(&buffer);
    T::deserialize(&mut reader)
}

pub fn erase<D, T>(device: &mut D, logical_addr: Addr) -> Result<(), Error>
where
    D: BlockDevice,
    T: Addressable + SerdeLen,
{
    let buf = [0u8; Block::LEN];
    let begin = T::LAYOUT.nth(logical_addr);
    for i in 0..T::SERDE_BLOCK_COUNT {
        device.write(begin + i as Addr, &buf)?;
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
        device.assert_write(4, Layout::DATA.nth(4), &[13u8; 452]);
    }
}
