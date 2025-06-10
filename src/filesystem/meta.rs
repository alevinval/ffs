use crate::filesystem::{
    Addr, Block, Deserializable, EraseFromDevice, Range, Serializable, StaticReadFromDevice,
    WriteToDevice,
};
use crate::io::{Reader, Write};
use crate::{BlockDevice, Error};

use super::{DataWriter, File, FreeBlockAllocator, Node};

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
    pub const RANGE: Range = Range::new(0, 1);
    const SIGNATURE: [u8; 2] = [0x13, 0x37];

    pub const fn new() -> Self {
        Meta {
            file_sector: File::RANGE.begin(),
            node_sector: Node::RANGE.begin(),
            free_sector: FreeBlockAllocator::RANGE.begin(),
            data_sector: DataWriter::RANGE.begin(),
            block_size: Block::LEN as u16,
        }
    }
}

impl Serializable for Meta {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        writer.write_u32(self.file_sector)?;
        writer.write_u32(self.node_sector)?;
        writer.write_u32(self.free_sector)?;
        writer.write_u32(self.data_sector)?;
        writer.write_u16(self.block_size)?;
        writer.write(&[0; 492])?;
        writer.write(&Self::SIGNATURE)?;
        Ok(())
    }
}

impl Deserializable<Meta> for Meta {
    fn deserialize(buf: &[u8]) -> Result<Meta, Error> {
        if buf[510..512] != Self::SIGNATURE {
            return Err(Error::Unsupported);
        }

        let mut r = Reader::new(buf);
        Ok(Meta {
            file_sector: r.read_u32()?,
            node_sector: r.read_u32()?,
            free_sector: r.read_u32()?,
            data_sector: r.read_u32()?,
            block_size: r.read_u16()?,
        })
    }
}

impl<D> WriteToDevice<D> for Meta
where
    D: BlockDevice,
{
    fn write_to_device(&self, out: &mut D) -> Result<(), Error> {
        let mut block = Block::new();
        self.serialize(&mut block.writer())?;

        let sector = Self::RANGE.begin();
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
        Meta::deserialize(&block)
    }
}

impl<D> EraseFromDevice<D> for Meta
where
    D: BlockDevice,
{
    fn erase_from_device(&self, device: &mut D) -> Result<(), Error> {
        device.write_block(Self::RANGE.begin(), &Block::new())
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

        assert_eq!(Err(Error::Unsupported), Meta::deserialize(&block));
    }

    #[test]
    fn serde_symmetry() -> Result<(), Error> {
        let mut block = Block::new();

        let expected = Meta::new();
        expected.serialize(&mut block.writer())?;
        let actual = Meta::deserialize(&block)?;
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
