use crate::{
    Addr, BlockDevice, Error,
    filesystem::{
        DataWriter, DirectoryTable, EraseFromDevice, File, FileName, FreeBlockAllocator,
        MAX_FILENAME_LEN, Meta, Node, NodeWriter, ReadFromDevice, StaticReadFromDevice,
        WriteToDevice,
        directory::{DirectoryEntry, EntryIter},
        file_name,
        node_writer::NodeRef,
    },
};

#[derive(Debug)]
pub struct Controller<D>
where
    D: BlockDevice,
{
    table: DirectoryTable,
    allocator: FreeBlockAllocator,
    device: D,
}

impl<D> Controller<D>
where
    D: BlockDevice,
{
    pub fn from(mut device: D) -> Result<Controller<D>, Error> {
        if Meta::read_from_device(&mut device)? != Meta::new() {
            return Err(Error::Unsupported);
        }
        let table = DirectoryTable::read_from_device(&mut device)?;
        Ok(Self { table, allocator: FreeBlockAllocator::new(), device })
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

        if self.table.file_exists(&file_name) {
            return Err(Error::FileAlreadyExists);
        }

        let entry = self.table.add_file(file_name)?;

        let mut block_addrs = [0 as Addr; Node::BLOCKS_PER_NODE];
        self.allocator.allocate_bytes(file_size, &mut block_addrs)?;
        let node = Node::new(file_size as u16, block_addrs);
        let file = File::new(entry.file_name().clone(), entry.file_addr());

        file.write_to_device(&mut self.device)?;
        NodeWriter::new(file.addr(), &node).write_to_device(&mut self.device)?;
        DataWriter::new(node.block_addrs(), data)
            .write(&mut self.device)
            .expect("cannot write data");
        self.allocator.write_to_device(&mut self.device)?;
        self.table.write_to_device(&mut self.device)?;

        Ok(())
    }

    pub fn delete(&mut self, file_name: &str) -> Result<(), Error> {
        let file_name = FileName::new(file_name)?;
        if let Some(entry) = self.table.find_file(&file_name) {
            let node_ref = NodeRef::new(entry.file_addr());
            let node = node_ref.read_from_device(&mut self.device)?;
            node.block_addrs().iter().for_each(|addr| self.allocator.release(*addr));
            node_ref.erase_from_device(&mut self.device)?;

            file.erase_from_device(&mut self.device)?;
            self.allocator.write_to_device(&mut self.device)?;

            return Ok(());
        }

        Err(Error::FileNotFound)
    }

    pub fn iter_files(&self) -> EntryIter {
        self.table.iter()
    }

    pub fn device(&mut self) -> &mut D {
        &mut self.device
    }
}
