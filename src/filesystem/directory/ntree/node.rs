use crate::{
    BlockDevice, Error,
    filesystem::{
        Addr, Deserializable, Layout, Name, SerdeLen, Serializable,
        block::Block,
        directory::{
            entry::{Entry, EntryKind},
            search::{binary_search, binary_search_mut},
        },
    },
    io::{Read, Reader, Write, Writer},
};

#[derive(Debug, PartialEq, Eq)]
pub struct TreeNode {
    entries: [Entry; Self::LEN],
}

impl TreeNode {
    const LAYOUT: Layout = Layout::TREE;

    pub const LEN: usize = 28;

    pub const fn new() -> Self {
        let entries = [const { Entry::empty() }; Self::LEN];
        Self { entries }
    }

    pub(super) const fn new_leaf() -> Self {
        let entries = [const { Entry::empty() }; Self::LEN];
        Self { entries }
    }

    pub fn insert(&mut self, name: &str, addr: Addr, kind: EntryKind) -> Result<Entry, Error> {
        let (_, entry) = self.find_unset().ok_or(Error::StorageFull)?;
        let name = Name::new(name)?;
        let value = Entry::new(name, addr, kind);
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

    pub fn iter_entries(&self) -> impl Iterator<Item = &Entry> {
        self.filter(|entry| entry.is_set())
    }

    pub fn iter_entries_mut(&mut self) -> impl Iterator<Item = &mut Entry> {
        self.entries.iter_mut().filter(|entry| entry.is_set())
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
            device.read(start_sector + i as Addr, chunk)?;
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
            device.write(start_sector + i as Addr, chunk)?;
        }
        Ok(())
    }
}

impl SerdeLen for TreeNode {
    const SERDE_LEN: usize = Self::LEN * Entry::SERDE_LEN;
}

impl Serializable for TreeNode {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<usize, Error> {
        let mut n = 0;
        for entry in &self.entries {
            n += entry.serialize(writer)?;
        }
        Ok(n)
    }
}

impl Deserializable<Self> for TreeNode {
    fn deserialize<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let mut entries = [const { Entry::empty() }; Self::LEN];
        for entry in entries.iter_mut() {
            *entry = Entry::deserialize(reader)?;
        }

        Ok(Self { entries })
    }
}
#[cfg(test)]
mod test {

    use crate::test_serde_symmetry;

    use super::*;

    test_serde_symmetry!(TreeNode, TreeNode::new());
}
