use crate::{
    BlockDevice, Error,
    filesystem::{
        Addr, Deserializable, Layout, SerdeLen, Serializable, block::Block,
        directory::file_ref::FileRef, name::Name,
    },
    io::{Read, Reader, Write, Writer},
};

#[derive(Debug, PartialEq, Eq)]
pub struct DirEntry {
    pub name: Name,
    pub dir_addrs: [Addr; Self::MAX_CHILD_DIRS],
    pub file_refs: [FileRef; Self::MAX_CHILD_FILES],
}

impl DirEntry {
    const LAYOUT: Layout = Layout::TREE;

    pub const MAX_CHILD_FILES: usize = 27;
    pub const MAX_CHILD_DIRS: usize = 16;

    pub const fn root() -> Self {
        Self::new(Name::empty())
    }

    pub const fn new(name: Name) -> Self {
        let dirs = [const { 0 }; Self::MAX_CHILD_DIRS];
        let file_refs = [const { FileRef::empty() }; Self::MAX_CHILD_FILES];
        Self { name, dir_addrs: dirs, file_refs }
    }

    pub fn load<D: BlockDevice>(device: &mut D, idx: Addr) -> Result<Self, Error> {
        let mut buffer = [0u8; Self::SERDE_BUFFER_LEN];
        let start_sector = Self::LAYOUT.nth(idx);

        for (i, chunk) in buffer.chunks_mut(Block::LEN).enumerate() {
            device.read_block(start_sector + i as Addr, chunk)?;
        }

        let mut reader = Reader::new(&buffer);
        Self::deserialize(&mut reader)
    }

    pub fn store<D>(&self, device: &mut D, idx: Addr) -> Result<(), Error>
    where
        D: BlockDevice,
    {
        let start_sector = Self::LAYOUT.nth(idx);
        let mut buffer = [0u8; Self::SERDE_BUFFER_LEN];
        let mut writer = Writer::new(&mut buffer);
        self.serialize(&mut writer)?;

        for (i, chunk) in buffer.chunks(Block::LEN).enumerate() {
            device.write_block(start_sector + i as Addr, chunk)?;
        }
        Ok(())
    }
}

impl SerdeLen for DirEntry {
    const SERDE_LEN: usize = Name::SERDE_LEN
        + Self::MAX_CHILD_DIRS * size_of::<Addr>()
        + Self::MAX_CHILD_FILES * FileRef::SERDE_LEN;
}

impl Serializable for DirEntry {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<usize, Error> {
        let mut n = self.name.serialize(writer)?;
        for addr in self.dir_addrs {
            n += writer.write_addr(addr)?;
        }
        for file_ref in &self.file_refs {
            n += file_ref.serialize(writer)?;
        }
        Ok(n)
    }
}

impl Deserializable<Self> for DirEntry {
    fn deserialize<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let name = Name::deserialize(reader)?;

        let mut dir_addrs = [0; Self::MAX_CHILD_DIRS];
        for dir in dir_addrs.iter_mut() {
            *dir = reader.read_addr()?;
        }

        let mut file_refs = [const { FileRef::empty() }; Self::MAX_CHILD_FILES];
        for file_ref in file_refs.iter_mut() {
            *file_ref = FileRef::deserialize(reader)?;
        }

        Ok(Self { name, dir_addrs, file_refs })
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
