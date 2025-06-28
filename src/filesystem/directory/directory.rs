use crate::{
    Addr, BlockDevice, Error,
    filesystem::{
        Block, Deserializable, FileName, Layout, MAX_FILES, Serializable, directory::FileEntry,
    },
};

#[derive(Debug, PartialEq, Eq)]
pub struct Directory {}

impl Directory {
    const LEN: usize = MAX_FILES;

    pub fn add_file<D>(&self, device: &mut D, file_name: FileName) -> Result<FileEntry, Error>
    where
        D: BlockDevice,
    {
        for idx in 0..Self::LEN as Addr {
            let mut entry = load_entry(device, idx)?;
            if !entry.is_valid() {
                entry.update(file_name, idx as Addr);
                store_entry(device, idx, &entry)?;
                return Ok(entry);
            }
        }
        Err(Error::StorageFull)
    }

    pub fn remove_file<D>(&self, device: &mut D, file_name: &FileName) -> Result<(), Error>
    where
        D: BlockDevice,
    {
        for idx in 0..Self::LEN as Addr {
            let entry = load_entry(device, idx)?;
            if entry.is_valid() && entry.name() == file_name {
                store_entry(device, idx, &FileEntry::default())?;
                return Ok(());
            }
        }
        Err(Error::FileNotFound)
    }

    pub fn find_file<D>(&self, device: &mut D, file_name: &FileName) -> Result<FileEntry, Error>
    where
        D: BlockDevice,
    {
        for idx in 0..Self::LEN as Addr {
            let entry = load_entry(device, idx)?;
            if entry.is_valid() && entry.name() == file_name {
                return Ok(entry);
            }
        }
        Err(Error::FileNotFound)
    }

    pub fn file_exists<D>(&self, device: &mut D, name: &FileName) -> bool
    where
        D: BlockDevice,
    {
        self.find_file(device, name).is_ok()
    }

    pub fn iter<'a, D>(&self, device: &'a mut D) -> EntryIter<'a, D>
    where
        D: BlockDevice,
    {
        EntryIter::new(device)
    }
}

fn load_entry<D>(device: &mut D, idx: Addr) -> Result<FileEntry, Error>
where
    D: BlockDevice,
{
    let mut block = Block::new();
    let sector = Layout::TABLE.nth(idx);
    device.read_block(sector, &mut block)?;
    FileEntry::deserialize(&mut block.reader())
}

fn store_entry<D>(device: &mut D, idx: Addr, entry: &FileEntry) -> Result<(), Error>
where
    D: BlockDevice,
{
    let mut block = Block::new();
    entry.serialize(&mut block.writer())?;
    let sector = Layout::TABLE.nth(idx);
    device.write_block(sector, &block)?;
    Ok(())
}

pub struct EntryIter<'a, D>
where
    D: BlockDevice,
{
    device: &'a mut D,
    pos: usize,
}

impl<'a, D> core::iter::Iterator for EntryIter<'a, D>
where
    D: BlockDevice,
{
    type Item = FileEntry;

    fn next(&mut self) -> Option<Self::Item> {
        while self.pos < Directory::LEN {
            if let Ok(entry) = load_entry(self.device, self.pos as Addr) {
                self.pos += 1;
                if !entry.is_valid() {
                    continue;
                }
                return Some(entry);
            }
            self.pos += 1;
        }
        None
    }
}

impl<'a, D> EntryIter<'a, D>
where
    D: BlockDevice,
{
    pub fn new(device: &'a mut D) -> Self {
        Self { device, pos: 0 }
    }
}

#[cfg(test)]
mod test {

    use crate::test_utils::MockDevice;

    use super::*;

    #[test]
    fn create_empty_directory_table() {
        let mut device = MockDevice::new();
        let table = Directory {};
        assert_eq!(None, table.iter(&mut device).next());
    }
}
