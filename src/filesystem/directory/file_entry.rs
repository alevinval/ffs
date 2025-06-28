use crate::{
    Addr, Error,
    filesystem::{Deserializable, FileName, SerdeLen, Serializable},
    io::{Read, Write},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    file_name: FileName,
    file_addr: Addr,
}

impl FileEntry {
    pub const fn empty() -> Self {
        Self { file_name: FileName::empty(), file_addr: 0 }
    }

    pub const fn new(file_name: FileName, file_addr: Addr) -> Self {
        Self { file_name, file_addr }
    }

    pub const fn name(&self) -> &FileName {
        &self.file_name
    }

    pub const fn is_valid(&self) -> bool {
        !self.file_name.is_empty()
    }

    pub const fn file_addr(&self) -> Addr {
        self.file_addr
    }
}

impl Default for FileEntry {
    fn default() -> Self {
        Self::empty()
    }
}

impl SerdeLen for FileEntry {
    const SERDE_LEN: usize = FileName::SERDE_LEN + size_of::<Addr>();
}

impl Serializable for FileEntry {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<usize, Error> {
        let mut n = self.file_name.serialize(writer)?;
        n += writer.write_u32(self.file_addr)?;
        Ok(n)
    }
}

impl Deserializable<FileEntry> for FileEntry {
    fn deserialize<R: Read>(reader: &mut R) -> Result<FileEntry, Error> {
        let file_name = FileName::deserialize(reader)?;
        let file_addr = reader.read_u32()?;
        Ok(FileEntry { file_name, file_addr })
    }
}

#[cfg(test)]
mod test {

    use super::*;

    use crate::filesystem::Block;

    #[test]
    fn serde_symmetry() {
        let mut block = Block::new();

        let expected = FileEntry::new("test_file".into(), 1);
        assert_eq!(Ok(FileEntry::SERDE_LEN), expected.serialize(&mut block.writer()));
        let actual = FileEntry::deserialize(&mut block.reader()).unwrap();

        assert_eq!(expected, actual);
    }
}
