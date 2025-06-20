use core::str;

use super::Addr;
use crate::filesystem::{
    Block, Deserializable, EraseFromDevice, FileName, MAX_FILES, Meta, Range, Serializable,
    WriteToDevice,
};
use crate::io::{Reader, Write};
use crate::{BlockDevice, Error};

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct File {
    name: FileName,
    addr: Addr,
}

impl File {
    pub const RANGE: Range = Meta::RANGE.next(MAX_FILES as Addr);

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
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        writer.write_u32(self.addr)?;
        self.name.serialize(writer)?;
        Ok(())
    }
}

impl Deserializable<File> for File {
    fn deserialize(buf: &[u8]) -> Result<File, Error> {
        let mut r = Reader::new(buf);
        let addr: u32 = r.read_u32()?;
        let name = FileName::deserialize(&buf[4..])?;
        Ok(File { name, addr })
    }
}

impl<D> WriteToDevice<D> for File
where
    D: BlockDevice,
{
    fn write_to_device(&self, out: &mut D) -> Result<(), Error> {
        let sector = Self::RANGE.nth(self.addr);
        let mut block = Block::new();
        self.serialize(&mut block.writer())?;
        out.write_block(sector, &block)
    }
}

impl<D> EraseFromDevice<D> for File
where
    D: BlockDevice,
{
    fn erase_from_device(&self, out: &mut D) -> Result<(), Error> {
        let sector = Self::RANGE.nth(self.addr);
        let block = Block::new();
        out.write_block(sector, &block)
    }
}

#[cfg(test)]
mod test {
    use crate::{filesystem::MAX_FILENAME_LEN, test_utils::MockDevice};

    use super::*;

    #[test]
    fn name_returns_correct_bytes() {
        let sut = File::new("abc123".into(), 42);
        assert_eq!(&FileName::new("abc123").unwrap(), sut.name());
    }

    #[test]
    fn deserialize_invalid_buffer_length_panics() {
        let buf = [128u8; 5 + MAX_FILENAME_LEN - 1];
        assert_eq!(
            Err(Error::BufferTooSmall { expected: 129, found: 128 }),
            File::deserialize(&buf)
        );
    }

    #[test]
    fn serde_symmetry() -> Result<(), Error> {
        let mut block = Block::new();
        let expected = File::new("test.txt".into(), 123);
        expected.serialize(&mut block.writer())?;
        let actual = File::deserialize(&block)?;
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
        out.assert_write(0, 124, &expected);

        Ok(())
    }

    #[test]
    fn erase_from_device() -> Result<(), Error> {
        let mut out = MockDevice::new();
        let sut = File::new("some-file.txt".into(), 123);
        sut.erase_from_device(&mut out)?;
        out.assert_write(0, 124, &[0; Block::LEN]);
        Ok(())
    }
}
