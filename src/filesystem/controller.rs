use crate::{
    Addr, BlockDevice, Error,
    filesystem::{
        DataWriter, EraseFromDevice, File, FileName, FreeBlockAllocator, MAX_FILENAME_LEN,
        MAX_FILES, Meta, Node, StaticReadFromDevice, WriteToDevice, node_writer::NodeWriter,
    },
};

#[derive(Debug)]
pub struct Controller<D>
where
    D: BlockDevice,
{
    entries: [Option<(File, Node)>; MAX_FILES],
    allocator: FreeBlockAllocator,
    file_count: Addr,
    device: D,
}

impl<D> Controller<D>
where
    D: BlockDevice,
{
    pub fn from(mut device: D) -> Result<Controller<D>, Error> {
        Meta::read_from_device(&mut device).map_err(|_| Error::Unsupported)?;
        Ok(Self {
            entries: [const { None }; MAX_FILES],
            allocator: FreeBlockAllocator::new(),
            file_count: 0,
            device,
        })
    }

    pub fn format(device: &mut D) -> Result<(), Error> {
        Meta::new().write_to_device(device)
    }

    pub fn create(&mut self, file_name: &str, data: &[u8]) -> Result<(), Error>
    where
        D: BlockDevice,
    {
        let file_name = FileName::new(file_name)?;
        let file_size = data.len();

        if file_size > Node::MAX_FILE_SIZE {
            return Err(Error::FileTooLarge);
        }

        if file_name.len() > MAX_FILENAME_LEN {
            return Err(Error::FileNameTooLong);
        }

        if self.file_count as usize > MAX_FILES {
            return Err(Error::StorageFull);
        }

        let mut block_addrs = [0 as Addr; Node::BLOCKS_PER_NODE];
        self.allocator.allocate_bytes(file_size, &mut block_addrs)?;

        let node = Node::new(file_size as u16, block_addrs);
        let file = File::new(file_name, self.file_count);

        file.write_to_device(&mut self.device)?;
        NodeWriter::new(file.addr(), &node).write_to_device(&mut self.device)?;
        DataWriter::new(node.block_addrs(), data)
            .write(&mut self.device)
            .expect("cannot write data");

        self.allocator.write_to_device(&mut self.device)?;

        let pos = self.file_count as usize;
        self.entries[pos] = Some((file, node));
        self.file_count += 1;

        Ok(())
    }

    pub fn delete(&mut self, filename: &str) -> Result<(), Error> {
        if let Some((file, node)) = self.find_file(filename) {
            node.block_addrs().iter().for_each(|addr| self.allocator.release(*addr));
            NodeWriter::new(file.addr(), &node).erase_from_device(&mut self.device)?;
            file.erase_from_device(&mut self.device)?;
            self.allocator.write_to_device(&mut self.device)?;

            self.entries[file.addr() as usize] = None;
            self.file_count -= 1;

            return Ok(());
        }

        Err(Error::FileNotFound)
    }

    pub fn iter_files(&self) -> FileIter {
        FileIter::new(&self.entries)
    }

    pub fn device(&mut self) -> &mut D {
        &mut self.device
    }

    fn find_file(&self, name: &str) -> Option<(File, Node)> {
        let name = FileName::new(name).ok()?;
        self.entries.iter().flatten().find(|(file, _)| file.name() == &name).cloned()
    }
}

pub struct FileIter<'a> {
    entries: &'a [Option<(File, Node)>],
    pos: usize,
}

impl<'a> core::iter::Iterator for FileIter<'a> {
    type Item = &'a File;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((file, _)) = self.entries[self.pos..].iter().flatten().next() {
            self.pos += 1;
            return Some(file);
        }
        None
    }
}

impl<'a> FileIter<'a> {
    pub fn new(entries: &'a [Option<(File, Node)>]) -> Self {
        Self { entries, pos: 0 }
    }
}
