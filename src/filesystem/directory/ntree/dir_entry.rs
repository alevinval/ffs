use crate::{
    BlockDevice, Error,
    filesystem::{
        Addr, Deserializable, Layout, Name, SerdeLen, Serializable, block::Block,
        directory::file_ref::FileRef,
    },
    io::{Read, Reader, Write, Writer},
};

#[derive(Debug, PartialEq, Eq)]
pub struct DirEntry {
    is_leaf: bool,
    is_empty: bool,
    edges: [FileRef; Self::MAX_EDGES],
}

impl DirEntry {
    const LAYOUT: Layout = Layout::TREE;

    pub const MAX_EDGES: usize = 29;

    pub const fn new() -> Self {
        let edges = [const { FileRef::empty() }; Self::MAX_EDGES];
        Self { is_empty: false, is_leaf: false, edges }
    }

    pub(super) const fn new_leaf() -> Self {
        let edges = [const { FileRef::empty() }; Self::MAX_EDGES];
        Self { is_empty: false, is_leaf: true, edges }
    }

    pub const fn is_empty(&self) -> bool {
        self.is_empty
    }

    pub const fn is_leaf(&self) -> bool {
        self.is_leaf
    }

    pub fn insert_node(&mut self, name: &str, addr: Addr) -> Result<FileRef, Error> {
        let (_, edge) = self.find_unset().ok_or(Error::StorageFull)?;
        let name = Name::new(name)?;
        *edge = FileRef::new(name, addr);
        Ok(edge.clone())
    }

    pub fn insert_file(&mut self, name: &str, addr: Addr) -> Result<FileRef, Error> {
        if !self.is_leaf {
            return Err(Error::Unsupported);
        }

        let (pos, edge) = self.find_unset().ok_or(Error::StorageFull)?;
        let name = Name::new(name)?;
        *edge = FileRef::new(name, addr * Self::MAX_EDGES as Addr + pos as Addr);
        Ok(edge.clone())
    }

    pub fn find(&self, name: &str) -> Option<&FileRef> {
        self.edges.iter().find(|r| r.name().as_str() == name)
    }

    pub fn find_mut(&mut self, name: &str) -> Option<&mut FileRef> {
        self.edges.iter_mut().find(|r| r.name().as_str() == name)
    }

    pub fn find_unset(&mut self) -> Option<(usize, &mut FileRef)> {
        self.edges.iter_mut().enumerate().find(|(_, r)| !r.is_set())
    }

    pub fn iter_set(&self) -> impl Iterator<Item = &FileRef> {
        self.filter(|r| r.is_set())
    }

    fn filter<P>(&self, predicate: P) -> impl Iterator<Item = &FileRef>
    where
        P: FnMut(&&FileRef) -> bool,
    {
        self.edges.iter().filter(predicate)
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
    const SERDE_LEN: usize = 1 + Self::MAX_EDGES * FileRef::SERDE_LEN;
}

impl Serializable for DirEntry {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<usize, Error> {
        let mut bitmap = 0u8;
        if !self.is_empty {
            bitmap |= 0b1;
        }
        if self.is_leaf {
            bitmap |= 0b1 << 1;
        }
        let mut n = writer.write_u8(bitmap)?;
        for edge in &self.edges {
            n += edge.serialize(writer)?;
        }
        Ok(n)
    }
}

impl Deserializable<Self> for DirEntry {
    fn deserialize<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let bitmap = reader.read_u8()?;
        let is_empty = bitmap & 0b1 == 0;
        let is_leaf = bitmap & (0b10) == 2;

        let mut edges = [const { FileRef::empty() }; Self::MAX_EDGES];
        for edge in edges.iter_mut() {
            *edge = FileRef::deserialize(reader)?;
        }

        Ok(Self { is_empty, is_leaf, edges })
    }
}
#[cfg(test)]
mod test {

    use crate::io::{Reader, Writer};

    use super::*;

    #[test]
    fn serde_symmetry() {
        assert_eq!(3, DirEntry::SERDE_BLOCK_COUNT);

        let expected = DirEntry::new();
        let mut buffer = [0u8; Block::LEN * DirEntry::SERDE_BLOCK_COUNT];
        let mut writer = Writer::new(&mut buffer);
        let n = expected.serialize(&mut writer).unwrap();
        assert_eq!(DirEntry::SERDE_LEN, n);

        let mut reader = Reader::new(&buffer);
        let actual = DirEntry::deserialize(&mut reader).unwrap();

        assert_eq!(expected, actual);
    }
}
