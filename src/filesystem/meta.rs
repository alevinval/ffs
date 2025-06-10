use crate::{
    filesystem::{
        Addr, Block, BlockDevice, Deserializable, EraseFromDevice, Error, Layout, Serializable,
        StaticReadFromDevice, WriteToDevice,
    },
    io::{Read, Write},
};

#[derive(PartialEq, Eq, Debug)]
pub struct Meta {
    file_sector: Addr,
    node_sector: Addr,
    free_sector: Addr,
    data_sector: Addr,
    block_size: u16,
}

impl Default for Meta {
    fn default() -> Self {
        Self::new()
    }
}

impl Meta {
    const SIGNATURE: [u8; 2] = [0x13, 0x37];

    pub const fn new() -> Self {
        Meta {
            file_sector: Layout::FILE.begin(),
            node_sector: Layout::NODE.begin(),
            free_sector: Layout::FREE.begin(),
            data_sector: Layout::DATA.begin(),
            block_size: Block::LEN as u16,
        }
    }
}

impl Serializable for Meta {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<usize, Error> {
        let mut n = writer.write_u32(self.file_sector)?;
        n += writer.write_u32(self.node_sector)?;
        n += writer.write_u32(self.free_sector)?;
        n += writer.write_u32(self.data_sector)?;
        n += writer.write_u16(self.block_size)?;
        n += writer.write(&[0; 492])?;
        n += writer.write(&Self::SIGNATURE)?;
        Ok(n)
    }
}

impl Deserializable<Meta> for Meta {
    fn deserialize<R: Read>(reader: &mut R) -> Result<Meta, Error> {
        let file_sector = reader.read_u32()?;
        let node_sector = reader.read_u32()?;
        let free_sector = reader.read_u32()?;
        let data_sector = reader.read_u32()?;
        let block_size = reader.read_u16()?;
        reader.read(&mut [0; 492])?;

        let mut signature = [0u8; 2];
        reader.read(&mut signature)?;
        if signature != Self::SIGNATURE {
            return Err(Error::Unsupported);
        }

        Ok(Meta { file_sector, node_sector, free_sector, data_sector, block_size })
    }
}

impl<D> WriteToDevice<D> for Meta
where
    D: BlockDevice,
{
    fn write_to_device(&self, out: &mut D) -> Result<(), Error> {
        let mut block = Block::new();
        self.serialize(&mut block.writer())?;

        let sector = Layout::META.begin();
        out.write_block(sector, &block)
    }
}

impl<D> StaticReadFromDevice<D> for Meta
where
    D: BlockDevice,
{
    type Item = Self;

    fn read_from_device(device: &mut D) -> Result<Self, Error> {
        let mut block = Block::new();
        device.read_block(0, &mut block)?;
        Meta::deserialize(&mut block.reader())
    }
}

impl<D> EraseFromDevice<D> for Meta
where
    D: BlockDevice,
{
    fn erase_from_device(&self, device: &mut D) -> Result<(), Error> {
        device.write_block(Layout::META.begin(), &Block::new())
    }
}

#[cfg(test)]
mod test {
    use crate::test_utils::MockDevice;

    use super::*;
    #[test]
    fn serialize_writes_signature() -> Result<(), Error> {
        let mut block = Block::new();
        let meta = Meta::new();
        meta.serialize(&mut block.writer())?;
        assert_eq!(&block[510..512], &Meta::SIGNATURE);
        Ok(())
    }

    #[test]
    fn deserialize_invalid_signature_panics() {
        let mut block = Block::new();
        let meta = Meta::new();
        meta.serialize(&mut block.writer()).unwrap();
        block[510] = 0x00; // Corrupt the signature

        assert_eq!(Err(Error::Unsupported), Meta::deserialize(&mut block.reader()));
    }

    #[test]
    fn serde_symmetry() -> Result<(), Error> {
        let mut block = Block::new();

        let expected = Meta::new();
        expected.serialize(&mut block.writer())?;
        let actual = Meta::deserialize(&mut block.reader())?;
        assert_eq!(expected, actual);

        Ok(())
    }

    #[test]
    fn erase_from_device() {
        let mut out = MockDevice::new();
        let sut = Meta::new();
        assert_eq!(Ok(()), sut.erase_from_device(&mut out));

        out.assert_write(0, 0, &[0u8; Block::LEN]);
    }
}
