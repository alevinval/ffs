use crate::{
    BlockDevice, Error,
    filesystem::{
        Addr, Deserializable, Layout, Name, SerdeLen, Serializable,
        block::Block,
        directory::{
            entry::Entry,
            search::{binary_search, binary_search_mut},
        },
    },
    io::{Read, Reader, Write, Writer},
};

#[derive(Debug, PartialEq, Eq)]
pub struct TreeNode {
    is_leaf: bool,
    is_empty: bool,
    entries: [Entry; Self::LEN],
}

impl TreeNode {
    const LAYOUT: Layout = Layout::TREE;

    pub const LEN: usize = 29;

    pub const fn new() -> Self {
        let entries = [const { Entry::empty() }; Self::LEN];
        Self { is_empty: false, is_leaf: false, entries }
    }

    pub(super) const fn new_leaf() -> Self {
        let entries = [const { Entry::empty() }; Self::LEN];
        Self { is_empty: false, is_leaf: true, entries }
    }

    pub const fn is_leaf(&self) -> bool {
        self.is_leaf
    }

    pub fn insert_node(&mut self, name: &str, addr: Addr) -> Result<Entry, Error> {
        assert!(!self.is_leaf, "Cannot insert node into a leaf node");
        let (_, entry) = self.find_unset().ok_or(Error::StorageFull)?;
        let name = Name::new(name)?;
        let value = Entry::new(name, addr);
        *entry = value.clone();
        self.entries.sort_by(|a, b| a.name().as_str().cmp(b.name().as_str()));
        Ok(value)
    }

    pub fn insert_file(&mut self, name: &str, addr: Addr) -> Result<Entry, Error> {
        assert!(self.is_leaf, "Cannot insert file into a non-leaf node");
        let (pos, entry) = self.find_unset().ok_or(Error::StorageFull)?;
        let name = Name::new(name)?;
        let value = Entry::new(name, addr * Self::LEN as Addr + pos as Addr);
        *entry = value.clone();
        self.entries.sort_by(|a, b| a.name().as_str().cmp(b.name().as_str()));
        Ok(value)
    }

    pub fn find(&self, name: &str) -> Option<&Entry> {
        binary_search(&self.entries, name, |entry| entry.name().as_str())
    }

    pub fn find_mut(&mut self, name: &str) -> Option<&mut Entry> {
        binary_search_mut(&mut self.entries, name, |entry| entry.name().as_str())
    }

    pub fn find_unset(&mut self) -> Option<(usize, &mut Entry)> {
        self.entries.iter_mut().enumerate().find(|(_, entry)| !entry.is_set())
    }

    pub fn iter_set(&self) -> impl Iterator<Item = &Entry> {
        self.filter(|entry| entry.is_set())
    }

    fn filter<P>(&self, predicate: P) -> impl Iterator<Item = &Entry>
    where
        P: FnMut(&&Entry) -> bool,
    {
        self.entries.iter().filter(predicate)
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

impl SerdeLen for TreeNode {
    const SERDE_LEN: usize = 1 + Self::LEN * Entry::SERDE_LEN;
}

impl Serializable for TreeNode {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<usize, Error> {
        let mut bitmap = 0u8;
        if !self.is_empty {
            bitmap |= 0b1;
        }
        if self.is_leaf {
            bitmap |= 0b1 << 1;
        }
        let mut n = writer.write_u8(bitmap)?;
        for entry in &self.entries {
            n += entry.serialize(writer)?;
        }
        Ok(n)
    }
}

impl Deserializable<Self> for TreeNode {
    fn deserialize<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let bitmap = reader.read_u8()?;
        let is_empty = bitmap & 0b1 == 0;
        let is_leaf = bitmap & (0b10) == 2;

        let mut entries = [const { Entry::empty() }; Self::LEN];
        for entry in entries.iter_mut() {
            *entry = Entry::deserialize(reader)?;
        }

        Ok(Self { is_empty, is_leaf, entries })
    }
}
#[cfg(test)]
mod test {

    use crate::io::{Reader, Writer};

    use super::*;

    #[test]
    fn serde_symmetry() {
        assert_eq!(3, TreeNode::SERDE_BLOCK_COUNT);

        let expected = TreeNode::new();
        let mut buffer = [0u8; Block::LEN * TreeNode::SERDE_BLOCK_COUNT];
        let mut writer = Writer::new(&mut buffer);
        let n = expected.serialize(&mut writer).unwrap();
        assert_eq!(TreeNode::SERDE_LEN, n);

        let mut reader = Reader::new(&buffer);
        let actual = TreeNode::deserialize(&mut reader).unwrap();

        assert_eq!(expected, actual);
    }
}
