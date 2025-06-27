use crate::{
    Addr, Error,
    filesystem::{Deserializable, FileName, SerdeLen, Serializable},
    io::{Read, Write},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    file_name: FileName,
    file_addr: Addr,
    is_valid: bool,
}

impl FileEntry {
    pub const fn empty() -> Self {
        Self { file_name: FileName::empty(), file_addr: 0, is_valid: false }
    }

    pub const fn new(file_name: FileName, file_addr: Addr) -> Self {
        Self { file_name, file_addr, is_valid: false }
    }

    pub fn update(&mut self, file_name: FileName, file_addr: Addr) {
        self.file_name = file_name;
        self.file_addr = file_addr;
        self.is_valid = true;
    }

    pub fn rename(&mut self, new_name: FileName) {
        self.file_name = new_name;
    }

    pub fn set_addr(&mut self, new_addr: Addr) {
        self.file_addr = new_addr;
    }

    pub const fn name(&self) -> &FileName {
        &self.file_name
    }

    pub const fn is_valid(&self) -> bool {
        self.is_valid
    }

    pub const fn file_addr(&self) -> Addr {
        self.file_addr
    }

    pub const fn file_name(&self) -> &FileName {
        &self.file_name
    }
}

impl Default for FileEntry {
    fn default() -> Self {
        Self::empty()
    }
}

impl SerdeLen for FileEntry {
    const SERDE_LEN: usize = FileName::SERDE_LEN + 5;
}

impl Serializable for FileEntry {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<usize, Error> {
        let mut n = self.file_name.serialize(writer)?;
        n += writer.write_u32(self.file_addr)?;
        n += writer.write_u8(self.is_valid as u8)?;
        Ok(n)
    }
}

impl Deserializable<FileEntry> for FileEntry {
    fn deserialize<R: Read>(reader: &mut R) -> Result<FileEntry, Error> {
        let file_name = FileName::deserialize(reader)?;
        let file_addr = reader.read_u32()?;
        let is_empty = reader.read_u8()? != 0;
        Ok(FileEntry { file_name, file_addr, is_valid: is_empty })
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
