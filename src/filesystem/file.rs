use core::str;

use crate::{
    Addr, BlockDevice, Error,
    filesystem::{Block, Deserializable, FileName, Layout, Serializable, WriteToDevice},
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
    use crate::{filesystem::MAX_FILENAME_LEN, io::Reader, test_utils::MockDevice};

    use super::*;

    #[test]
    fn name_returns_correct_bytes() {
        let sut = File::new("abc123".into(), 42);
        assert_eq!(&FileName::new("abc123").unwrap(), sut.name());
    }

    #[test]
    fn deserialize_invalid_buffer_length_panics() {
        let mut reader = Reader::new(&[128u8; 5 + MAX_FILENAME_LEN - 1]);
        assert_eq!(
            Err(Error::BufferTooSmall { expected: 133, found: 132 }),
            File::deserialize(&mut reader)
        );
    }

    #[test]
    fn serde_symmetry() -> Result<(), Error> {
        let mut block = Block::new();
        let expected = File::new("test.txt".into(), 123);
        expected.serialize(&mut block.writer())?;
        let actual = File::deserialize(&mut block.reader())?;
        assert_eq!(expected, actual);
        Ok(())
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
