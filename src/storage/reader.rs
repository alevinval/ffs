use crate::{BlockDevice, Error, Meta, alloc_block_buffer, serde::Deserializable};

pub struct Reader {}

impl Reader {
    pub fn read_metadata<D>(device: &mut D) -> Result<Meta, Error>
    where
        D: BlockDevice,
    {
        let mut buf = alloc_block_buffer();
        device.read_block(0, &mut buf)?;
        Meta::deserialize(&buf)
    }
}
