use crate::{
    Addr, BlockDevice, Error,
    filesystem::{
        Block, Deserializable, FileName, Layout, MAX_FILES, SerdeLen, Serializable,
        StaticReadFromDevice, WriteToDevice, directory::FileEntry,
    },
};

#[derive(Debug, PartialEq, Eq)]
pub struct Directory {
    entries: [FileEntry; Self::LEN],
}

impl Directory {
    pub const SLOTS: usize = Block::LEN / FileEntry::SERDE_LEN;

    const LEN: usize = MAX_FILES;

    pub fn new() -> Self {
        Self { entries: [const { FileEntry::empty() }; Self::LEN] }
    }
    pub fn add_file(&mut self, file_name: FileName) -> Result<&FileEntry, Error> {
        for (addr, entry) in self.entries.iter_mut().enumerate() {
            if !entry.is_valid() {
                entry.update(file_name, addr as Addr);
                return Ok(entry);
            }
        }
        Err(Error::StorageFull)
    }

    pub fn remove_file(&mut self, file_name: &FileName) -> Result<(), Error> {
        for entry in self.entries.iter_mut() {
            if entry.is_valid() && entry.name() == file_name {
                *entry = FileEntry::default();
                return Ok(());
            }
        }
        Err(Error::FileNotFound)
    }

    pub fn find_file(&self, file_name: &FileName) -> Option<&FileEntry> {
        self.entries.iter().find(|e| e.is_valid() && e.name() == file_name)
    }

    pub fn find_file_mut(&mut self, file_name: &FileName) -> Option<&mut FileEntry> {
        self.entries.iter_mut().find(|e| e.is_valid() && e.name() == file_name)
    }

    pub fn list_files(&self) -> impl Iterator<Item = &FileEntry> {
        self.entries.iter().filter(|e| e.is_valid())
    }

    pub fn rename_file(&mut self, old_name: &FileName, new_name: FileName) -> bool {
        if let Some(entry) = self.find_file_mut(old_name) {
            entry.rename(new_name);
            return true;
        }
        false
    }

    pub fn update_file_addr(&mut self, name: &FileName, new_addr: Addr) -> bool {
        if let Some(entry) = self.find_file_mut(name) {
            entry.set_addr(new_addr);
            return true;
        }
        false
    }

    pub fn file_exists(&self, name: &FileName) -> bool {
        self.find_file(name).is_some()
    }

    pub fn iter(&self) -> EntryIter {
        EntryIter::new(&self.entries)
    }
}

impl<D> WriteToDevice<D> for Directory
where
    D: BlockDevice,
{
    fn write_to_device(&self, out: &mut D) -> Result<(), Error> {
        for (i, chunk) in self.entries.chunks(Self::SLOTS).enumerate() {
            let sector = Layout::TABLE.nth(i as Addr);
            let mut block = Block::new();
            let mut writer = block.writer();
            for entry in chunk {
                entry.serialize(&mut writer)?;
            }
            out.write_block(sector, &block)?;
        }
        Ok(())
    }
}

impl<D> StaticReadFromDevice<D> for Directory
where
    D: BlockDevice,
{
    type Item = Directory;

    fn read_from_device(device: &mut D) -> Result<Self::Item, Error> {
        let mut n = 0;
        let mut table = Directory::new();
        let mut block = Block::new();
        for sector in Layout::TABLE.range_sectors() {
            device.read_block(sector, &mut block)?;
            let mut reader = block.reader();
            for _ in 0..Self::SLOTS {
                if n >= Self::LEN {
                    break;
                }
                table.entries[n] = FileEntry::deserialize(&mut reader)?;
                n += 1;
            }
        }
        Ok(table)
    }
}

pub struct EntryIter<'a> {
    entries: &'a [FileEntry],
    pos: usize,
}

impl<'a> core::iter::Iterator for EntryIter<'a> {
    type Item = &'a FileEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(entry) = self.entries[self.pos..].iter().find(|entry| entry.is_valid()) {
            self.pos += 1;
            return Some(entry);
        }
        None
    }
}

impl<'a> EntryIter<'a> {
    pub fn new(entries: &'a [FileEntry]) -> Self {
        Self { entries, pos: 0 }
    }
}

#[cfg(test)]
mod test {

    use crate::test_utils::MockDevice;

    use super::*;

    #[test]
    fn create_empty_directory_table() {
        let table = Directory::new();
        assert_eq!(table.entries.len(), Directory::LEN);
        assert_eq!(None, table.iter().next());
    }

    #[test]
    fn write_then_read_from_device() {
        let mut device = MockDevice::new();
        let mut expected = Directory::new();
        assert!(expected.add_file("one".into()).is_ok());
        assert!(expected.add_file("two".into()).is_ok());

        assert_eq!(Ok(()), expected.write_to_device(&mut device));

        let actual = Directory::read_from_device(&mut device).unwrap();
        assert_eq!(expected, actual);
    }
}
