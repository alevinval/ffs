use crate::{
    Error,
    filesystem::{
        Addr, Addressable, Deserializable, Layout, Name, SerdeLen, Serializable,
        tree::entry::{Entry, Kind},
    },
    io::{Read, Write},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeNode {
    entries: [Entry; Self::LEN],
}

impl Default for TreeNode {
    fn default() -> Self {
        Self::new()
    }
}

impl TreeNode {
    pub const LEN: usize = 30;

    pub const fn new() -> Self {
        let entries = [const { Entry::empty() }; Self::LEN];
        Self { entries }
    }

    pub(super) const fn new_leaf() -> Self {
        let entries = [const { Entry::empty() }; Self::LEN];
        Self { entries }
    }

    pub fn insert(&mut self, name: &str, addr: Addr, kind: Kind) -> Result<Entry, Error> {
        let (_, entry) = self.find_unset().ok_or(Error::DirectoryFull)?;
        let name = Name::new(name)?;
        let value = Entry::new(name, addr, kind);
        *entry = value.clone();
        self.entries.sort_by(|a, b| a.name().as_str().cmp(b.name().as_str()));
        Ok(value)
    }

    pub const fn get(&self, pos: usize) -> &Entry {
        &self.entries[pos]
    }

    pub const fn get_mut(&mut self, pos: usize) -> &mut Entry {
        &mut self.entries[pos]
    }

    pub fn find_index(&self, name: &str) -> Option<usize> {
        binary_search_index(&self.entries, name, |entry| entry.name().as_str())
    }

    pub fn find(&self, name: &str) -> Option<&Entry> {
        self.find_index(name).and_then(|idx| self.entries.get(idx))
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
}

fn binary_search_index<T, K>(list: &[T], value: &K, get_key: impl Fn(&T) -> &K) -> Option<usize>
where
    K: Ord + ?Sized,
{
    let mut low = 0;
    let mut high = list.len();
    while low < high {
        let mid = usize::midpoint(low, high);
        match get_key(&list[mid]).cmp(value) {
            core::cmp::Ordering::Less => low = mid + 1,
            core::cmp::Ordering::Equal => return Some(mid),
            core::cmp::Ordering::Greater => high = mid,
        }
    }
    None
}

impl Addressable for TreeNode {
    const LAYOUT: Layout = Layout::TREE;
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
        for entry in &mut entries {
            *entry = Entry::deserialize(reader)?;
        }

        Ok(Self { entries })
    }
}
#[cfg(test)]
mod tests {

    use std::format;

    use crate::test_serde_symmetry;

    use super::*;

    test_serde_symmetry!(TreeNode, TreeNode::new());

    #[test]
    fn test_insert_full_node() {
        let mut sut = TreeNode::new();
        for i in 0..=TreeNode::LEN {
            let addr = Addr::from(i as u32);
            let kind = if i % 2 == 0 { Kind::File } else { Kind::Dir };
            sut.insert(&format!("entry-{i}"), addr, kind).expect("should insert entry");
        }

        assert_eq!(Err(Error::DirectoryFull), sut.insert("extra-entry", 100 as Addr, Kind::File));
    }
}
