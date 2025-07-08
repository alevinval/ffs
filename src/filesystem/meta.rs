use crate::{
    filesystem::{
        Addr, Block, BlockDevice, Deserializable, EraseFrom, Error, Layout, LoadFromStatic,
        SerdeLen, Serializable, Store,
    },
    io::{Read, Write},
};

#[derive(PartialEq, Eq, Debug)]
pub struct Meta {
    tree_bitmap: Addr,
    tree_sector: Addr,
    file_sector: Addr,
    node_sector: Addr,
    data_bitmap: Addr,
    data_sector: Addr,
    block_size: u16,
    signature: [u8; 2],
}

impl Default for Meta {
    fn default() -> Self {
        Self::new()
    }
}

impl Meta {
    const SIGNATURE: [u8; 2] = [0x13, 0x37];

    pub const fn new() -> Self {
        Self {
            tree_bitmap: Layout::TREE_BITMAP.begin,
            tree_sector: Layout::TREE.begin,
            file_sector: Layout::FILE.begin,
            node_sector: Layout::NODE.begin,
            data_bitmap: Layout::DATA_BITMAP.begin,
            data_sector: Layout::DATA.begin,
            block_size: Block::LEN as u16,
            signature: Self::SIGNATURE,
        }
    }
}

impl SerdeLen for Meta {
    const SERDE_LEN: usize = Block::LEN;
}

impl Serializable for Meta {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<usize, Error> {
        let mut n = writer.write_addr(self.tree_bitmap)?;
        n += writer.write_addr(self.tree_sector)?;
        n += writer.write_addr(self.file_sector)?;
        n += writer.write_addr(self.node_sector)?;
        n += writer.write_addr(self.data_bitmap)?;
        n += writer.write_addr(self.data_sector)?;
        n += writer.write_u16(self.block_size)?;
        n += writer.write(&[0; 484])?;
        n += writer.write(&Self::SIGNATURE)?;
        Ok(n)
    }
}

impl Deserializable<Self> for Meta {
    fn deserialize<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let tree_bitmap = reader.read_addr()?;
        let tree_sector = reader.read_addr()?;
        let file_sector = reader.read_addr()?;
        let node_sector = reader.read_addr()?;
        let data_bitmap = reader.read_addr()?;
        let data_sector = reader.read_addr()?;
        let block_size = reader.read_u16()?;
        reader.read(&mut [0; 484])?;
        let mut signature = [0u8; 2];
        reader.read(&mut signature)?;

        Ok(Self {
            tree_bitmap,
            tree_sector,
            file_sector,
            node_sector,
            data_bitmap,
            data_sector,
            block_size,
            signature,
        })
    }
}

impl<D> Store<D> for Meta
where
    D: BlockDevice,
{
    fn store(&self, device: &mut D) -> Result<(), Error> {
        let mut block = Block::new();
        self.serialize(&mut block.writer())?;

        let sector = Layout::META.begin;
        device.write(sector, &block)
    }
}

impl<D> LoadFromStatic<D> for Meta
where
    D: BlockDevice,
{
    type Item = Self;

    fn load_from(device: &mut D) -> Result<Self, Error> {
        let mut block = Block::new();
        device.read(0, &mut block)?;
        Self::deserialize(&mut block.reader())
    }
}

impl<D> EraseFrom<D> for Meta
where
    D: BlockDevice,
{
    fn erase_from(&self, device: &mut D) -> Result<(), Error> {
        device.write(Layout::META.begin, &Block::new())
    }
}

#[cfg(test)]
mod tests {
    use crate::{test_serde_symmetry, test_utils::MockDevice};

    use super::*;

    test_serde_symmetry!(Meta, Meta::new());

    #[test]
    fn write_to_device_then_read() {
        let mut device = MockDevice::new();
        let expected = Meta::new();
        assert_eq!(Ok(()), expected.store(&mut device));
        assert_eq!(Ok(expected), Meta::load_from(&mut device));
    }

    #[test]
    fn erase_from_device() {
        let mut device = MockDevice::new();
        let sut = Meta::new();
        assert_eq!(Ok(()), sut.erase_from(&mut device));

        device.assert_write(0, 0, &[0u8; Block::LEN]);
    }
}
