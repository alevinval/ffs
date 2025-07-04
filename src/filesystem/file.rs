use crate::{
    BlockDevice, Error,
    filesystem::{
        Addr, Addressable, Block, Deserializable, Layout, Name, SerdeLen, Serializable, Store,
    },
    io::{Read, Write},
};

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct File {
    name: Name,
    node_addr: Addr,
}

impl File {
    pub const fn new(name: Name, node_addr: Addr) -> Self {
        Self { name, node_addr }
    }

    pub const fn node_addr(&self) -> Addr {
        self.node_addr
    }
}

impl SerdeLen for File {
    const SERDE_LEN: usize = 4 + Name::SERDE_LEN;
}

impl Serializable for File {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<usize, Error> {
        let mut n = writer.write_addr(self.node_addr)?;
        n += self.name.serialize(writer)?;
        Ok(n)
    }
}

impl Deserializable<Self> for File {
    fn deserialize<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let node_addr = reader.read_addr()?;
        let name = Name::deserialize(reader)?;
        Ok(Self { name, node_addr })
    }
}

impl Addressable for File {
    fn layout() -> Layout {
        Layout::FILE
    }
}

impl<D> Store<D> for File
where
    D: BlockDevice,
{
    fn store(&self, device: &mut D) -> Result<(), Error> {
        let sector = Layout::FILE.nth(self.node_addr);
        let mut block = Block::new();
        self.serialize(&mut block.writer())?;
        device.write(sector, &block)
    }
}

#[cfg(test)]
mod test {
    use crate::{test_serde_symmetry, test_utils::MockDevice};

    use super::*;

    test_serde_symmetry!(File, File::new("text.txt".into(), 123));

    #[test]
    fn write_to_device() -> Result<(), Error> {
        let mut device = MockDevice::new();
        let sut = File::new("some-file.txt".into(), 123);
        sut.store(&mut device)?;

        let mut expected = Block::new();
        sut.serialize(&mut expected.writer())?;
        device.assert_write(0, Layout::FILE.nth(123), &expected);

        Ok(())
    }
}
