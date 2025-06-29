use crate::{
    Addr, BlockDevice, Error,
    filesystem::{
        Deserializable, Layout, SerdeLen, Serializable, block::Block, directory::FileEntry,
        file_name::FileName,
    },
    io::{Read, Reader, Write, Writer},
};

#[derive(Debug, PartialEq, Eq)]
pub struct DirEntry {
    pub name: FileName,
    pub dirs: [Addr; Self::MAX_CHILD_DIRS],
    pub files: [FileEntry; Self::MAX_CHILD_FILES],
}

impl DirEntry {
    pub const MAX_CHILD_FILES: usize = 27;
    pub const MAX_CHILD_DIRS: usize = 16;

    pub fn root() -> Self {
        Self::new("".into())
    }

    pub const fn new(name: FileName) -> Self {
        let dirs = [const { 0 }; Self::MAX_CHILD_DIRS];
        let files = [const { FileEntry::empty() }; Self::MAX_CHILD_FILES];
        Self { name, dirs, files }
    }

    pub fn load<D: BlockDevice>(device: &mut D, idx: Addr) -> Result<DirEntry, Error> {
        let range = Layout::BTREE;
        let mut buffer = [0u8; DirEntry::SERDE_BUFFER_LEN];
        let start_sector = range.nth(idx);

        for (i, chunk) in buffer.chunks_mut(Block::LEN).enumerate() {
            device.read_block(start_sector + i as Addr, chunk)?;
        }

        let mut reader = Reader::new(&buffer);
        DirEntry::deserialize(&mut reader)
    }

    pub fn store<D>(&self, device: &mut D, idx: Addr) -> Result<(), Error>
    where
        D: BlockDevice,
    {
        let range = Layout::BTREE;
        let start_sector = range.nth(idx);
        let mut buffer = [0u8; DirEntry::SERDE_BUFFER_LEN];
        let mut writer = Writer::new(&mut buffer);
        self.serialize(&mut writer)?;

        for (i, chunk) in buffer.chunks(Block::LEN).enumerate() {
            device.write_block(start_sector + i as Addr, chunk)?;
        }
        Ok(())
    }
}

impl SerdeLen for DirEntry {
    const SERDE_LEN: usize = FileName::SERDE_LEN
        + Self::MAX_CHILD_DIRS * size_of::<Addr>()
        + Self::MAX_CHILD_FILES * FileEntry::SERDE_LEN;
}

impl Serializable for DirEntry {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<usize, Error> {
        let mut n = self.name.serialize(writer)?;
        for child in self.dirs {
            n += writer.write_u32(child)?;
        }
        for file in &self.files {
            n += file.serialize(writer)?;
        }
        Ok(n)
    }
}

impl Deserializable<DirEntry> for DirEntry {
    fn deserialize<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let name = FileName::deserialize(reader)?;

        let mut dirs = [0; Self::MAX_CHILD_DIRS];
        for dir in dirs.iter_mut() {
            *dir = reader.read_u32()?;
        }

        let mut files = [const { FileEntry::empty() }; Self::MAX_CHILD_FILES];
        for file in files.iter_mut() {
            *file = FileEntry::deserialize(reader)?;
        }

        Ok(Self { name, dirs, files })
    }
}
#[cfg(test)]
mod test {

    use crate::io::{Reader, Writer};

    use super::*;

    #[test]
    fn serde_symmetry() {
        let expected = DirEntry::root();
        let mut buffer = [0u8; Block::LEN * DirEntry::SERDE_BLOCK_COUNT];
        let mut writer = Writer::new(&mut buffer);
        let n = expected.serialize(&mut writer).unwrap();
        assert_eq!(DirEntry::SERDE_LEN, n);

        let mut reader = Reader::new(&buffer);
        let actual = DirEntry::deserialize(&mut reader).unwrap();

        assert_eq!(expected, actual);
    }
}
