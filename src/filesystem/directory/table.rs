use crate::{
    Addr, BlockDevice, Error,
    filesystem::{
        Block, Deserializable, FileName, Layout, Serializable, StaticReadFromDevice, WriteToDevice,
        directory::DirectoryEntry,
    },
};

#[derive(Debug, PartialEq, Eq)]
pub struct DirectoryTable {
    entries: [DirectoryEntry; Self::LEN],
}

impl DirectoryTable {
    pub const SLOTS: usize = Block::LEN / DirectoryEntry::SERIALIZED_LEN;

    const LEN: usize = Layout::TABLE.len() as usize;

    pub fn new() -> Self {
        Self { entries: [const { DirectoryEntry::empty() }; Self::LEN] }
    }
    pub fn add_file(&mut self, file_name: FileName) -> Result<&DirectoryEntry, Error> {
        for (addr, entry) in self.entries.iter_mut().enumerate() {
            if entry.is_empty() {
                entry.update(file_name, addr as Addr);
                return Ok(entry);
            }
        }
        Err(Error::StorageFull)
    }

    pub fn remove_file(&mut self, file_name: &FileName) -> Result<(), Error> {
        for entry in self.entries.iter_mut() {
            if !entry.is_empty() && entry.name() == file_name {
                *entry = DirectoryEntry::default();
                return Ok(());
            }
        }
        Err(Error::FileNotFound)
    }

    pub fn find_file(&self, file_name: &FileName) -> Option<&DirectoryEntry> {
        self.entries.iter().find(|e| !e.is_empty() && e.name() == file_name)
    }

    pub fn find_file_mut(&mut self, file_name: &FileName) -> Option<&mut DirectoryEntry> {
        self.entries.iter_mut().find(|e| !e.is_empty() && e.name() == file_name)
    }

    pub fn list_files(&self) -> impl Iterator<Item = &DirectoryEntry> {
        self.entries.iter().filter(|e| !e.is_empty())
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

impl<D> WriteToDevice<D> for DirectoryTable
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

impl<D> StaticReadFromDevice<D> for DirectoryTable
where
    D: BlockDevice,
{
    type Item = DirectoryTable;

    fn read_from_device(device: &mut D) -> Result<Self::Item, Error> {
        let mut n = 0;
        let mut table = DirectoryTable::new();
        let mut block = Block::new();
        for sector in Layout::TABLE.range_sectors() {
            device.read_block(sector, &mut block)?;
            let mut reader = block.reader();
            for _ in 0..Self::SLOTS {
                if n >= Self::LEN {
                    break;
                }
                table.entries[n] = DirectoryEntry::deserialize(&mut reader)?;
                n += 1;
            }
        }
        Ok(table)
    }
}

pub struct EntryIter<'a> {
    entries: &'a [DirectoryEntry],
    pos: usize,
}

impl<'a> core::iter::Iterator for EntryIter<'a> {
    type Item = &'a DirectoryEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(entry) =
            self.entries[self.pos..].iter().filter(|entry| !entry.is_empty()).next()
        {
            self.pos += 1;
            return Some(entry);
        }
        None
    }
}

impl<'a> EntryIter<'a> {
    pub fn new(entries: &'a [DirectoryEntry]) -> Self {
        Self { entries, pos: 0 }
    }
}

#[cfg(test)]
mod test {

    use crate::test_utils::MockDevice;

    use super::*;

    #[test]
    fn write_then_read_from_device() {
        let mut device = MockDevice::new();
        let mut expected = DirectoryTable::new();
        assert!(expected.add_file("one".into()).is_ok());
        assert!(expected.add_file("two".into()).is_ok());

        assert_eq!(Ok(()), expected.write_to_device(&mut device));

        let actual = DirectoryTable::read_from_device(&mut device).unwrap();
        assert_eq!(expected, actual);
    }
}
