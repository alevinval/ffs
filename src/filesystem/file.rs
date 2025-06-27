use core::str;

use crate::{
    Addr, BlockDevice, Error,
    filesystem::{Block, Deserializable, FileName, Layout, SerdeLen, Serializable, WriteToDevice},
    io::{Read, Write},
};

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct File {
    name: FileName,
    addr: Addr,
}

impl File {
    pub const fn new(name: FileName, addr: Addr) -> Self {
        File { name, addr }
    }

    pub fn name(&self) -> &FileName {
        &self.name
    }

    pub fn name_str(&self) -> &str {
        self.name.as_str()
    }

    pub const fn addr(&self) -> Addr {
        self.addr
    }
}

impl SerdeLen for File {
    const SERDE_LEN: usize = 4 + FileName::SERDE_LEN;
}

impl Serializable for File {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<usize, Error> {
        let mut n = writer.write_u32(self.addr)?;
        n += self.name.serialize(writer)?;
        Ok(n)
    }
}

impl Deserializable<File> for File {
    fn deserialize<R: Read>(reader: &mut R) -> Result<File, Error> {
        let addr: u32 = reader.read_u32()?;
        let name = FileName::deserialize(reader)?;
        Ok(File { name, addr })
    }
}

impl<D> WriteToDevice<D> for File
where
    D: BlockDevice,
{
    fn write_to_device(&self, out: &mut D) -> Result<(), Error> {
        let sector = Layout::FILE.nth(self.addr);
        let mut block = Block::new();
        self.serialize(&mut block.writer())?;
        out.write_block(sector, &block)
    }
}

#[cfg(test)]
mod test {
    use crate::test_utils::MockDevice;

    use super::*;

    #[test]
    fn name_returns_correct_bytes() {
        let sut = File::new("abc123".into(), 42);
        assert_eq!(&FileName::new("abc123").unwrap(), sut.name());
    }

    #[test]
    fn serde_symmetry() {
        let mut block = Block::new();

        let expected = File::new("test.txt".into(), 123);
        assert_eq!(Ok(File::SERDE_LEN), expected.serialize(&mut block.writer()));
        let actual = File::deserialize(&mut block.reader()).unwrap();

        assert_eq!(expected, actual);
    }

    #[test]
    fn write_to_device() -> Result<(), Error> {
        let mut out = MockDevice::new();
        let sut = File::new("some-file.txt".into(), 123);
        sut.write_to_device(&mut out)?;

        let mut expected = Block::new();
        sut.serialize(&mut expected.writer())?;
        out.assert_write(0, Layout::FILE.nth(123), &expected);

        Ok(())
    }
}
